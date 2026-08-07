pub mod credentials;
pub mod keyring;
pub mod onboarding;

pub use credentials::{CredentialError, CredentialLoader};
pub use keyring::{KeyringError, KeyringStore, MockKeyringStore};
pub use onboarding::{OnboardingManager, OnboardingStatus};
use cust_config_types::Config;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartialConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load(cli_override: PartialConfig) -> Result<Config, anyhow::Error> {
        let mut config = Config::default();

        // 1. User config in ~/.cust/config.toml or ~/.config/cust/config.toml
        if let Some(home) = dirs::home_dir() {
            let user_paths = [
                home.join(".cust").join("config.toml"),
                home.join(".config").join("cust").join("config.toml"),
            ];

            for path in &user_paths {
                if path.exists() {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        if let Ok(partial) = toml::from_str::<PartialConfig>(&content) {
                            apply_partial(&mut config, partial);
                            break;
                        }
                    }
                }
            }
        }

        // 2. Project config in ./.cust/config.toml
        let project_path = Path::new(".cust").join("config.toml");
        if project_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&project_path) {
                if let Ok(partial) = toml::from_str::<PartialConfig>(&content) {
                    apply_partial(&mut config, partial);
                }
            }
        }

        // 3. Environment variables
        if let Ok(prov) = std::env::var("CUST_PROVIDER") {
            if !prov.trim().is_empty() {
                config.provider = prov.trim().to_string();
            }
        }
        if let Ok(m) = std::env::var("CUST_MODEL") {
            if !m.trim().is_empty() {
                config.model = m.trim().to_string();
            }
        }
        if let Ok(url) = std::env::var("CUST_BASE_URL") {
            if !url.trim().is_empty() {
                config.base_url = Some(url.trim().to_string());
            }
        }
        if let Ok(key) = std::env::var("CUST_API_KEY") {
            if !key.trim().is_empty() {
                config.api_key = Some(key.trim().to_string());
            }
        }

        // 4. CLI flags override
        apply_partial(&mut config, cli_override);

        // If api_key is still missing, attempt to resolve via CredentialLoader
        if config.api_key.is_none() {
            let key = CredentialLoader::resolve_api_key(&config.provider)?;
            config.api_key = Some(key);
        }

        Ok(config)
    }
}

fn apply_partial(config: &mut Config, partial: PartialConfig) {
    if let Some(p) = partial.provider {
        if !p.trim().is_empty() {
            config.provider = p;
        }
    }
    if let Some(m) = partial.model {
        if !m.trim().is_empty() {
            config.model = m;
        }
    }
    if let Some(k) = partial.api_key {
        if !k.trim().is_empty() {
            config.api_key = Some(k);
        }
    }
    if let Some(u) = partial.base_url {
        if !u.trim().is_empty() {
            config.base_url = Some(u);
        }
    }
}
