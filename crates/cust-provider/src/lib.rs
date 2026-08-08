pub mod anthropic;
pub mod message;
pub mod capabilities;
pub mod failover;
pub mod generation;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use capabilities::{ProviderCapabilities, RemoteCompactionSupport};
pub use failover::ProviderFailoverGroup;
pub use generation::{GenerationManager, ModelGeneration};
pub use openai::OpenAIProvider;
pub use message::{ContentBlock, Message, Role};

use cust_config_types::{Config, ModelCapabilities};
use futures_util::Stream;
use std::pin::Pin;

pub type TextStream = Pin<Box<dyn Stream<Item = Result<String, anyhow::Error>> + Send>>;

pub enum ProviderClient {
    OpenAI(OpenAIProvider),
    Anthropic(AnthropicProvider),
}

impl ProviderClient {
    pub fn from_config(config: &Config) -> Result<Self, anyhow::Error> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing API key for provider '{}'", config.provider))?;

        let prov_str = config.provider.to_lowercase();
        let model = if config.model == "gpt-4o" && prov_str != "openai" {
            match prov_str.as_str() {
                "anthropic" | "claude" => "claude-3-5-sonnet-20241022".to_string(),
                "xai" => "grok-2-latest".to_string(),
                "mistral" => "mistral-small-latest".to_string(),
                _ => config.model.clone(),
            }
        } else {
            config.model.clone()
        };

        match prov_str.as_str() {
            "anthropic" | "claude" => Ok(Self::Anthropic(AnthropicProvider::new(
                api_key,
                model,
                config.base_url.clone(),
            ))),
            "openai" => Ok(Self::OpenAI(OpenAIProvider::new(
                api_key,
                model,
                config.base_url.clone(),
            ))),
            "xai" => {
                let base_url = config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.x.ai/v1".to_string());
                Ok(Self::OpenAI(OpenAIProvider::new(
                    api_key,
                    model,
                    Some(base_url),
                )))
            }
            "mistral" => {
                let base_url = config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.mistral.ai/v1".to_string());
                Ok(Self::OpenAI(OpenAIProvider::new(
                    api_key,
                    model,
                    Some(base_url),
                )))
            }
            "openrouter" => {
                let base_url = config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
                Ok(Self::OpenAI(OpenAIProvider::new(
                    api_key,
                    config.model.clone(),
                    Some(base_url),
                )))
            }
            _ => {
                // Default to OpenAI-compatible interface for custom or unknown providers
                Ok(Self::OpenAI(OpenAIProvider::new(
                    api_key,
                    config.model.clone(),
                    config.base_url.clone(),
                )))
            }
        }
    }

    pub fn capabilities(&self) -> ModelCapabilities {
        match self {
            Self::OpenAI(p) => p.capabilities(),
            Self::Anthropic(p) => p.capabilities(),
        }
    }

    pub fn stream_chat(&self, messages: Vec<Message>) -> TextStream {
        match self {
            Self::OpenAI(p) => p.stream_chat(messages),
            Self::Anthropic(p) => p.stream_chat(messages),
        }
    }

    /// Convenience: create a user message from a string prompt.
    pub fn stream_chat_prompt(&self, prompt: &str) -> TextStream {
        self.stream_chat(vec![Message::user(prompt)])
    }
}
