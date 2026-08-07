use cust_exec::sandbox::{SandboxProfile, is_self_protected};
use std::path::Path;

#[test]
fn test_self_protection_blocks_git_hooks() {
    assert!(is_self_protected(Path::new(
        "/home/user/project/.git/hooks/pre-commit"
    )));
    assert!(is_self_protected(Path::new(
        "C:\\Users\\dev\\project\\.git\\hooks\\post-merge"
    )));
}

#[test]
fn test_self_protection_blocks_ssh_keys() {
    assert!(is_self_protected(Path::new("/home/user/.ssh/id_rsa")));
}

#[test]
fn test_self_protection_blocks_shell_profiles() {
    assert!(is_self_protected(Path::new("/home/user/.bashrc")));
    assert!(is_self_protected(Path::new("/home/user/.zshrc")));
}

#[test]
fn test_self_protection_allows_normal_paths() {
    assert!(!is_self_protected(Path::new(
        "/home/user/project/src/main.rs"
    )));
    assert!(!is_self_protected(Path::new(
        "C:\\Users\\dev\\project\\src\\lib.rs"
    )));
}

#[test]
fn test_sandbox_off_still_blocks_self_protected() {
    let sandbox = SandboxProfile::Off;
    let cwd = Path::new("C:\\Users\\dev\\project");
    let target = Path::new("C:\\Users\\dev\\project\\.git\\hooks\\pre-commit");

    let result = sandbox.check_permission(cwd, true, Some(target));
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("self-protected directory")
    );
}
