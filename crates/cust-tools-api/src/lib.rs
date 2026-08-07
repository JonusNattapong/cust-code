use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Availability {
    Direct,
    CodeMode,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionRequest {
    None,
    ReadPath(PathBuf),
    WritePath(PathBuf),
    Execute(String),
    Network(String),
    Custom(String),
}

impl std::fmt::Display for PermissionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "No permission required"),
            Self::ReadPath(p) => write!(f, "Read access to {}", p.display()),
            Self::WritePath(p) => write!(f, "Write access to {}", p.display()),
            Self::Execute(cmd) => write!(f, "Execution of command '{cmd}'"),
            Self::Network(url) => write!(f, "Network access to {url}"),
            Self::Custom(desc) => write!(f, "{desc}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult<T = serde_json::Value> {
    pub ok: bool,
    pub summary: String,
    pub data: Option<T>,
}

impl<T> ToolResult<T> {
    pub fn success(summary: impl Into<String>, data: Option<T>) -> Self {
        Self {
            ok: true,
            summary: summary.into(),
            data,
        }
    }

    pub fn failure(summary: impl Into<String>) -> Self {
        Self {
            ok: false,
            summary: summary.into(),
            data: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Permission denied: {0}")]
    Denied(String),
    #[error("Tool execution failed: {0}")]
    Failed(#[from] anyhow::Error),
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn spec(&self) -> ToolSpec;
    fn permission(&self, args: &serde_json::Value) -> PermissionRequest;
    fn availability(&self) -> Availability;
    fn call<'a>(
        &'a self,
        args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'a>>;
}
