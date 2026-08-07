use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub path: PathBuf,
    pub script_path: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Discovery Budget: inspired by mistral-vibe ADR 0007 which mandates
// "deterministic and cheap" extension discovery. We enforce a startup time
// budget so that slow filesystem scans never block the main event loop.
// ---------------------------------------------------------------------------

/// Result of a budgeted discovery run.
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    /// Skills that were discovered within the time budget.
    pub skills: HashMap<String, Skill>,
    /// Directories whose scan was skipped because the budget expired.
    pub skipped_dirs: Vec<PathBuf>,
    /// Total wall-clock time spent discovering skills.
    pub elapsed: Duration,
}

pub struct SkillLoader {
    search_dirs: Vec<PathBuf>,
    /// Maximum wall-clock time for the entire discovery sweep. If `None`, no
    /// budget is enforced (unbounded scan).
    pub discovery_budget: Option<Duration>,
}

impl SkillLoader {
    pub fn default_loader() -> Self {
        let mut dirs = vec![
            PathBuf::from("./.cust/skills"),
            PathBuf::from("./.claude/skills"),
            PathBuf::from("./.codex/skills"),
        ];
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".cust").join("skills"));
            dirs.push(home.join(".claude").join("skills"));
        }
        Self {
            search_dirs: dirs,
            // 200 ms default budget — keeps startup path snappy.
            discovery_budget: Some(Duration::from_millis(200)),
        }
    }

    /// Create a loader with a custom time budget.
    pub fn with_budget(mut self, budget: Duration) -> Self {
        self.discovery_budget = Some(budget);
        self
    }

    /// Create a loader with no time budget (unbounded scan).
    pub fn unbounded(mut self) -> Self {
        self.discovery_budget = None;
        self
    }

    /// Discover skills with the configured time budget.
    pub fn discover_skills_budgeted(&self) -> DiscoveryResult {
        let start = Instant::now();
        let mut skills = HashMap::new();
        let mut skipped_dirs = Vec::new();

        for dir in &self.search_dirs {
            // Check budget before scanning each directory.
            if let Some(budget) = self.discovery_budget {
                if start.elapsed() >= budget {
                    skipped_dirs.push(dir.clone());
                    continue;
                }
            }

            if !dir.exists() {
                continue;
            }

            Self::scan_directory(dir, &mut skills);
        }

        DiscoveryResult {
            skills,
            skipped_dirs,
            elapsed: start.elapsed(),
        }
    }

    /// Original unbounded discovery (for backward compatibility).
    pub fn discover_skills(&self) -> HashMap<String, Skill> {
        let mut skills = HashMap::new();
        for dir in &self.search_dirs {
            if !dir.exists() {
                continue;
            }
            Self::scan_directory(dir, &mut skills);
        }
        skills
    }

    fn scan_directory(dir: &PathBuf, skills: &mut HashMap<String, Skill>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.file_name().and_then(|f| f.to_str()) == Some("SKILL.md") {
                    if let Some(skill) = Self::parse_skill(&path) {
                        skills.entry(skill.name.clone()).or_insert(skill);
                    }
                } else if path.is_dir() {
                    let skill_md = path.join("SKILL.md");
                    if skill_md.exists() {
                        if let Some(skill) = Self::parse_skill(&skill_md) {
                            skills.entry(skill.name.clone()).or_insert(skill);
                        }
                    }
                }
            }
        }
    }

    fn parse_skill(path: &PathBuf) -> Option<Skill> {
        let content = std::fs::read_to_string(path).ok()?;
        let mut lines = content.lines();

        let mut name = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed-skill")
            .to_string();
        let mut description = String::new();

        for line in &mut lines {
            if line.starts_with("# ") {
                name = line.trim_start_matches("# ").trim().to_string();
            } else if line.starts_with("> ") || (!line.is_empty() && description.is_empty()) {
                description = line.trim_start_matches("> ").trim().to_string();
                break;
            }
        }

        let mut script_path = None;
        if let Some(parent) = path.parent() {
            for script_name in &["run.sh", "main.py", "run.bat", "script.py"] {
                let candidate = parent.join(script_name);
                if candidate.exists() {
                    script_path = Some(candidate);
                    break;
                }
            }
        }

        Some(Skill {
            name,
            description,
            instructions: content,
            path: path.clone(),
            script_path,
        })
    }

    pub fn format_progressive_summary(skills: &HashMap<String, Skill>) -> String {
        if skills.is_empty() {
            return "No active skills found.".to_string();
        }
        let mut text = String::from("### Available Skills (Progressive Disclosure)\n");
        for (name, skill) in skills {
            text.push_str(&format!("- **{}**: {}\n", name, skill.description));
        }
        text
    }
}
