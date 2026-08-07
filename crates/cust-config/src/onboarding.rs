use cust_config_types::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingStatus {
    pub is_configured: bool,
    pub active_provider: String,
    pub active_model: String,
    pub has_api_key: bool,
}

pub struct OnboardingManager;

impl OnboardingManager {
    pub fn check_status(config: &Config) -> OnboardingStatus {
        let has_api_key = config.api_key.as_ref().is_some_and(|k| !k.is_empty());
        OnboardingStatus {
            is_configured: has_api_key,
            active_provider: config.provider.clone(),
            active_model: config.model.clone(),
            has_api_key,
        }
    }

    pub fn welcome_banner(status: &OnboardingStatus) -> String {
        if status.is_configured {
            format!(
                " Welcome to cust code! Active LLM: {} ({})",
                status.active_model, status.active_provider
            )
        } else {
            format!(
                " Welcome to cust code!\n\
                 [Notice] No API key detected for provider '{}'.\n\
                 Please configure your credentials in ~/.clew/provider.json or set ENVIRONMENT variables.",
                status.active_provider
            )
        }
    }
}
