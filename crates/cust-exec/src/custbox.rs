use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustboxMount {
    pub source: PathBuf,
    pub target: PathBuf,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustboxConfig {
    pub mounts: Vec<CustboxMount>,
    pub read_only_paths: Vec<PathBuf>,
    pub egress_rules: Vec<String>,
    pub max_memory_mb: Option<usize>,
}

impl CustboxConfig {
    pub fn is_path_read_only(&self, path: &std::path::Path) -> bool {
        for ro in &self.read_only_paths {
            if path.starts_with(ro) {
                return true;
            }
        }
        for mount in &self.mounts {
            if mount.read_only && path.starts_with(&mount.target) {
                return true;
            }
        }
        false
    }
}
