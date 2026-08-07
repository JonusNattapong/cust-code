use cust_core::GitTracker;

#[test]
fn test_fast_git_status() {
    let cwd = std::env::current_dir().unwrap();
    let status = GitTracker::fast_status(&cwd);
    assert!(status.is_repo);
    assert!(status.branch.is_some());
}
