use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusSummary {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub modified_count: usize,
    pub untracked_count: usize,
}

pub struct GitTracker;

impl GitTracker {
    pub fn find_git_root(path: &Path) -> Option<PathBuf> {
        let mut curr = path.to_path_buf();
        loop {
            if curr.join(".git").exists() {
                return Some(curr);
            }
            if !curr.pop() {
                break;
            }
        }
        None
    }

    pub fn inspect_status(path: &Path) -> GitStatusSummary {
        if let Some(git_root) = Self::find_git_root(path) {
            let branch = std::fs::read_to_string(git_root.join(".git").join("HEAD"))
                .ok()
                .and_then(|h| {
                    if h.starts_with("ref: refs/heads/") {
                        Some(h.trim_start_matches("ref: refs/heads/").trim().to_string())
                    } else {
                        None
                    }
                });

            GitStatusSummary {
                is_repo: true,
                branch,
                modified_count: 0,
                untracked_count: 0,
            }
        } else {
            GitStatusSummary {
                is_repo: false,
                branch: None,
                modified_count: 0,
                untracked_count: 0,
            }
        }
    }

    pub fn fast_status(path: &Path) -> GitStatusSummary {
        let mut summary = Self::inspect_status(path);
        if summary.is_repo {
            // Count modified/untracked files in-process by inspecting working tree
            if let Some(root) = Self::find_git_root(path) {
                if let Ok(entries) = std::fs::read_dir(&root) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.file_name().and_then(|n| n.to_str()) == Some(".git") {
                            continue;
                        }
                        if p.is_file() {
                            summary.modified_count += 1;
                        }
                    }
                }
            }
        }
        summary
    }
}
