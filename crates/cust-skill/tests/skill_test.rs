use cust_skill::SkillLoader;
use std::time::Duration;

#[test]
fn test_budgeted_discovery_completes_within_budget() {
    let loader = SkillLoader::default_loader().with_budget(Duration::from_secs(5));
    let result = loader.discover_skills_budgeted();

    // Should complete within the budget (5 seconds is generous)
    assert!(result.elapsed < Duration::from_secs(5));
    // No directories should be skipped with such a generous budget
    assert!(result.skipped_dirs.is_empty());
}

#[test]
fn test_unbounded_discovery_returns_skills() {
    let loader = SkillLoader::default_loader().unbounded();
    // Just verify it runs without panic
    let _skills = loader.discover_skills();
}
