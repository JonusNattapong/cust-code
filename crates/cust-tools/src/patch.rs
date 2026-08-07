use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Hunk {
    AddFile {
        path: PathBuf,
        content: String,
    },
    DeleteFile {
        path: PathBuf,
    },
    UpdateFile {
        path: PathBuf,
        old_lines: Vec<String>,
        new_lines: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppliedChange {
    Added { path: PathBuf, content: String },
    Deleted { path: PathBuf, old_content: String },
    Updated { path: PathBuf, old_content: String, new_content: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppliedPatchDelta {
    pub changes: Vec<AppliedChange>,
    pub is_exact: bool,
}

pub struct PatchEngine;

impl PatchEngine {
    /// Fuzzy seek for matching lines in a sequence.
    pub fn seek_sequence(lines: &[String], pattern: &[String], start: usize) -> Option<usize> {
        if pattern.is_empty() {
            return Some(start);
        }
        if pattern.len() > lines.len() {
            return None;
        }

        for i in start..=(lines.len() - pattern.len()) {
            if lines[i..(i + pattern.len())] == *pattern {
                return Some(i);
            }
        }
        None
    }

    /// Apply a list of hunks to the filesystem rooted at `base_dir`.
    pub fn apply_hunks(hunks: &[Hunk], base_dir: &Path) -> Result<AppliedPatchDelta, anyhow::Error> {
        let mut delta = AppliedPatchDelta {
            changes: Vec::new(),
            is_exact: true,
        };

        for hunk in hunks {
            match hunk {
                Hunk::AddFile { path, content } => {
                    let full_path = base_dir.join(path);
                    if let Some(parent) = full_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&full_path, content)?;
                    delta.changes.push(AppliedChange::Added {
                        path: path.clone(),
                        content: content.clone(),
                    });
                }
                Hunk::DeleteFile { path } => {
                    let full_path = base_dir.join(path);
                    let old_content = std::fs::read_to_string(&full_path).unwrap_or_default();
                    if full_path.exists() {
                        std::fs::remove_file(&full_path)?;
                    }
                    delta.changes.push(AppliedChange::Deleted {
                        path: path.clone(),
                        old_content,
                    });
                }
                Hunk::UpdateFile { path, old_lines, new_lines } => {
                    let full_path = base_dir.join(path);
                    let old_content = std::fs::read_to_string(&full_path)?;
                    let current_lines: Vec<String> = old_content.lines().map(String::from).collect();

                    if let Some(pos) = Self::seek_sequence(&current_lines, old_lines, 0) {
                        let mut updated_lines = current_lines[..pos].to_vec();
                        updated_lines.extend(new_lines.iter().cloned());
                        updated_lines.extend(current_lines[(pos + old_lines.len())..].iter().cloned());

                        let new_content = updated_lines.join("\n");
                        std::fs::write(&full_path, &new_content)?;
                        delta.changes.push(AppliedChange::Updated {
                            path: path.clone(),
                            old_content,
                            new_content,
                        });
                    } else {
                        delta.is_exact = false;
                        return Err(anyhow::anyhow!(
                            "Patch hunk matching failed for file '{}'",
                            path.display()
                        ));
                    }
                }
            }
        }

        Ok(delta)
    }
}
