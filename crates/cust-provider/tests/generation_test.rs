use cust_config_types::Config;
use cust_provider::GenerationManager;

#[test]
fn test_model_generation_isolation() {
    let cfg1 = Config {
        provider: "mistral".to_string(),
        model: "mistral-small-latest".to_string(),
        api_key: Some("key1".to_string()),
        base_url: None,
    };
    let mgr = GenerationManager::new(cfg1);

    let gen1 = mgr.current_generation();
    assert_eq!(gen1.id, 1);

    let cfg2 = Config {
        provider: "openai".to_string(),
        model: "gpt-4o".to_string(),
        api_key: Some("key2".to_string()),
        base_url: None,
    };
    let gen2 = mgr.update_generation(cfg2);
    assert_eq!(gen2.id, 2);

    // Verify gen1 holds immutable snapshot of cfg1
    assert_eq!(gen1.config.provider, "mistral");
    assert_eq!(gen2.config.provider, "openai");
}
