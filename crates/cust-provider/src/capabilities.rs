use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RemoteCompactionSupport {
    #[default]
    Unsupported,
    V1,
    V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub namespace_tools: bool,
    pub image_generation: bool,
    pub web_search: bool,
    pub remote_compaction: RemoteCompactionSupport,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            namespace_tools: true,
            image_generation: true,
            web_search: true,
            remote_compaction: RemoteCompactionSupport::V2,
        }
    }
}

impl ProviderCapabilities {
    pub fn for_provider(provider_name: &str) -> Self {
        match provider_name.to_lowercase().as_str() {
            "openai" => Self {
                namespace_tools: true,
                image_generation: true,
                web_search: true,
                remote_compaction: RemoteCompactionSupport::V2,
            },
            "anthropic" | "claude" => Self {
                namespace_tools: true,
                image_generation: false,
                web_search: true,
                remote_compaction: RemoteCompactionSupport::V1,
            },
            "ollama" | "lmstudio" | "local" => Self {
                namespace_tools: false,
                image_generation: false,
                web_search: false,
                remote_compaction: RemoteCompactionSupport::Unsupported,
            },
            _ => Self::default(),
        }
    }
}
