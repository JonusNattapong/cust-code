use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Mistral,
    XAI,
    OpenRouter,
    Custom,
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAI => write!(f, "openai"),
            Self::Anthropic => write!(f, "anthropic"),
            Self::Mistral => write!(f, "mistral"),
            Self::XAI => write!(f, "xai"),
            Self::OpenRouter => write!(f, "openrouter"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub chat: bool,
    pub vision: bool,
    pub tool_calling: bool,
    pub streaming: bool,
    pub max_context: Option<usize>,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            chat: true,
            vision: false,
            tool_calling: false,
            streaming: true,
            max_context: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            api_key: None,
            base_url: None,
        }
    }
}
