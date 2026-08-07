use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CredentialError {
    #[error("API key for provider '{provider}' was not found.\nInspected sources:\n{}", inspected_sources.iter().map(|s| format!("  - {s}")).collect::<Vec<_>>().join("\n"))]
    MissingKey {
        provider: String,
        inspected_sources: Vec<String>,
    },
}

pub struct CredentialLoader;

impl CredentialLoader {
    /// Resolve an API key for the given provider name.
    /// Checks environment variables, `.env`, `~/.clew/provider.json`, and `~/.clew/.credentials.json`.
    pub fn resolve_api_key(provider: &str) -> Result<String, CredentialError> {
        let mut inspected = Vec::new();

        // 1. Env vars
        let env_var_names = match provider.to_lowercase().as_str() {
            "openai" => vec!["OPENAI_API_KEY", "CUST_API_KEY"],
            "anthropic" | "claude" => vec!["ANTHROPIC_API_KEY", "CLAUDE_API_KEY", "CUST_API_KEY"],
            "mistral" => vec!["MISTRAL_API_KEY", "CUST_API_KEY"],
            "xai" => vec!["XAI_API_KEY", "CUST_API_KEY"],
            "openrouter" => vec!["OPENROUTER_API_KEY", "CUST_API_KEY"],
            _ => vec!["CUST_API_KEY"],
        };

        for var_name in &env_var_names {
            inspected.push(format!("environment variable ${var_name}"));
            if let Ok(val) = std::env::var(var_name) {
                if !val.trim().is_empty() {
                    return Ok(val.trim().to_string());
                }
            }
        }

        // 2. Local `.env` file
        let cwd_env = Path::new(".env");
        inspected.push(format!("file {}", cwd_env.display()));
        if cwd_env.exists() {
            if let Ok(iter) = dotenvy::from_path_iter(cwd_env) {
                for item in iter.flatten() {
                    if env_var_names.contains(&item.0.as_str()) && !item.1.trim().is_empty() {
                        return Ok(item.1.trim().to_string());
                    }
                }
            }
        }

        // 3. ~/.clew/provider.json
        if let Some(home) = dirs::home_dir() {
            let provider_json_path = home.join(".clew").join("provider.json");
            inspected.push(format!("file {}", provider_json_path.display()));
            if provider_json_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&provider_json_path) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(api_keys) = v.get("apiKeys").and_then(|k| k.as_object()) {
                            if let Some(key_val) = api_keys.get(provider).and_then(|s| s.as_str()) {
                                if !key_val.trim().is_empty() {
                                    return Ok(key_val.trim().to_string());
                                }
                            }
                        }
                    }
                }
            }

            // 4. ~/.clew/.credentials.json
            let creds_json_path = home.join(".clew").join(".credentials.json");
            inspected.push(format!("file {}", creds_json_path.display()));
            if creds_json_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&creds_json_path) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                        if provider.to_lowercase() == "anthropic"
                            || provider.to_lowercase() == "claude"
                        {
                            if let Some(token) = v
                                .pointer("/claudeAiOauth/accessToken")
                                .and_then(|s| s.as_str())
                            {
                                if !token.trim().is_empty() {
                                    return Ok(token.trim().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(CredentialError::MissingKey {
            provider: provider.to_string(),
            inspected_sources: inspected,
        })
    }
}
