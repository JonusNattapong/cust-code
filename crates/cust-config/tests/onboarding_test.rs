use cust_config::{OnboardingManager, OnboardingStatus};
use cust_config_types::Config;

#[test]
fn test_onboarding_check_status() {
    let mut cfg = Config::default();
    cfg.provider = "mistral".to_string();
    cfg.model = "mistral-small-latest".to_string();
    cfg.api_key = None;

    let status = OnboardingManager::check_status(&cfg);
    assert!(!status.is_configured);
    assert_eq!(status.active_provider, "mistral");

    cfg.api_key = Some("key123".to_string());
    let status_ok = OnboardingManager::check_status(&cfg);
    assert!(status_ok.is_configured);
}

#[test]
fn test_onboarding_welcome_banner() {
    let status = OnboardingStatus {
        is_configured: true,
        active_provider: "openai".to_string(),
        active_model: "gpt-4o".to_string(),
        has_api_key: true,
    };
    let banner = OnboardingManager::welcome_banner(&status);
    assert!(banner.contains("gpt-4o"));
}
