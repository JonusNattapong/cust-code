//! File-based memory storage with progressive disclosure.
//!
//! Layout (under `~/.cust-code/memory/`):
//! ```text
//! ~/.cust-code/memory/
//!   MEMORY.md                         # Searchable registry of knowledge
//!   {workspace_hash}/
//!     memory_summary.md               # Always loaded into system prompt (first line: v1)
//!     MEMORY.md                       # Project-level curated knowledge
//!     skills/<skill-name>/SKILL.md    # Reusable procedures
//!     rollout_summaries/*.md          # Per-rollout recaps
//! ```

use std::fs;
use std::path::{Path, PathBuf};

/// Compute a short workspace hash from a path (first 8 hex chars of blake3).
pub fn workspace_hash(cwd: &Path) -> String {
    let path_str = cwd.to_string_lossy().as_bytes().to_vec();
    let hash = blake3::hash(&path_str);
    hash.to_hex()[..8].to_string()
}

/// Root directory for all memory (~/.cust-code/memory/).
pub fn memory_root() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".cust-code").join("memory"))
}

/// Workspace-scoped memory directory.
pub fn workspace_memory_dir(cwd: &Path) -> Option<PathBuf> {
    memory_root().map(|root| root.join(workspace_hash(cwd)))
}

/// Ensure the memory directory tree exists for a workspace.
pub fn init_workspace(cwd: &Path) -> Option<PathBuf> {
    let dir = workspace_memory_dir(cwd)?;
    fs::create_dir_all(&dir).ok()?;
    fs::create_dir_all(dir.join("skills")).ok()?;
    fs::create_dir_all(dir.join("rollout_summaries")).ok()?;
    Some(dir)
}

/// Read a memory file, returning None if missing or unreadable.
pub fn read_file(dir: &Path, filename: &str) -> Option<String> {
    fs::read_to_string(dir.join(filename)).ok()
}

/// Write a memory file, creating parent directories as needed.
pub fn write_file(dir: &Path, filename: &str, content: &str) -> bool {
    let path = dir.join(filename);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, content).is_ok()
}

/// Append content to a memory file (creates if missing).
pub fn append_file(dir: &Path, filename: &str, content: &str) -> bool {
    let path = dir.join(filename);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    use std::io::Write;
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| f.write_all(content.as_bytes()))
        .is_ok()
}

/// Check if a memory_summary.md exists and starts with "v1".
pub fn is_v1(dir: &Path) -> bool {
    read_file(dir, "memory_summary.md")
        .map(|c| c.starts_with("v1\n") || c.starts_with("v1\r\n"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_workspace_hash_deterministic() {
        let a = workspace_hash(Path::new("/home/user/project"));
        let b = workspace_hash(Path::new("/home/user/project"));
        assert_eq!(a.len(), 8);
        assert_eq!(a, b);
    }

    #[test]
    fn test_workspace_hash_different_for_different_paths() {
        let a = workspace_hash(Path::new("/home/user/project-a"));
        let b = workspace_hash(Path::new("/home/user/project-b"));
        assert_ne!(a, b);
    }

    #[test]
    fn test_init_and_read_write() {
        let dir = tempdir().unwrap();
        let cwd = dir.path().join("myproject");
        fs::create_dir_all(&cwd).unwrap();

        // Override memory_root to use tempdir
        let mem_dir = dir.path().join("memory");
        fs::create_dir_all(&mem_dir).unwrap();
        fs::create_dir_all(mem_dir.join("skills")).unwrap();
        fs::create_dir_all(mem_dir.join("rollout_summaries")).unwrap();

        // write + read
        write_file(&mem_dir, "MEMORY.md", "# Test\n\n- item one\n");
        let content = read_file(&mem_dir, "MEMORY.md").unwrap();
        assert!(content.contains("item one"));

        // append
        append_file(&mem_dir, "MEMORY.md", "- item two\n");
        let content = read_file(&mem_dir, "MEMORY.md").unwrap();
        assert!(content.contains("item two"));

        // is_v1
        write_file(&mem_dir, "memory_summary.md", "v1\n\n## Profile\n");
        assert!(is_v1(&mem_dir));
    }
}
