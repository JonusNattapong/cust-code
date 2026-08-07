use cust_tools::{Hunk, PatchEngine};
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_seek_sequence() {
    let lines = vec![
        "fn main() {".to_string(),
        "    println!(\"hello\");".to_string(),
        "}".to_string(),
    ];
    let pattern = vec!["    println!(\"hello\");".to_string()];
    assert_eq!(PatchEngine::seek_sequence(&lines, &pattern, 0), Some(1));
}

#[test]
fn test_apply_hunks_add_and_update() {
    let dir = tempdir().unwrap();
    let file_path = PathBuf::from("test.rs");

    let hunks = vec![
        Hunk::AddFile {
            path: file_path.clone(),
            content: "line1\nline2\nline3".to_string(),
        },
        Hunk::UpdateFile {
            path: file_path.clone(),
            old_lines: vec!["line2".to_string()],
            new_lines: vec!["line2_modified".to_string()],
        },
    ];

    let delta = PatchEngine::apply_hunks(&hunks, dir.path()).unwrap();
    assert_eq!(delta.changes.len(), 2);

    let updated_content = std::fs::read_to_string(dir.path().join(&file_path)).unwrap();
    assert!(updated_content.contains("line2_modified"));
}
