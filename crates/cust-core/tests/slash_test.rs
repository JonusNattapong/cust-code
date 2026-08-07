use cust_core::SlashRegistry;

#[test]
fn test_slash_registry_parse_and_match() {
    let registry = SlashRegistry::new();

    // Parse input
    let (cmd, args) = registry.parse_input("/goal 1-3").unwrap();
    assert_eq!(cmd, "goal");
    assert_eq!(args, "1-3");

    // Match prefix
    let matches = registry.match_prefix("/re");
    assert!(matches.iter().any(|c| c.name == "rewind"));
    assert!(matches.iter().any(|c| c.name == "refine"));
}
