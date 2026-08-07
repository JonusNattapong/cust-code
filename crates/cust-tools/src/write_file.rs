use cust_tools_api::{Availability, PermissionRequest, Tool, ToolError, ToolResult, ToolSpec};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub struct WriteFileTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct WriteFileArgs {
    pub path: String,
    pub content: String,
}

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_file".to_string(),
            description: "Write content to a file on the filesystem.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    fn permission(&self, args: &serde_json::Value) -> PermissionRequest {
        if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
            PermissionRequest::WritePath(PathBuf::from(path))
        } else {
            PermissionRequest::None
        }
    }

    fn availability(&self) -> Availability {
        Availability::Both
    }

    fn call<'a>(
        &'a self,
        args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let write_args: WriteFileArgs = serde_json::from_value(args)
                .map_err(|e| ToolError::Failed(anyhow::anyhow!("Invalid arguments: {e}")))?;

            let path = PathBuf::from(&write_args.path);
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        ToolError::Failed(anyhow::anyhow!(
                            "Failed to create directory {}: {e}",
                            parent.display()
                        ))
                    })?;
                }
            }

            tokio::fs::write(&path, &write_args.content)
                .await
                .map_err(|e| {
                    ToolError::Failed(anyhow::anyhow!(
                        "Failed to write file {}: {e}",
                        path.display()
                    ))
                })?;

            let summary = format!(
                "Successfully wrote {} bytes to {}",
                write_args.content.len(),
                path.display()
            );
            Ok(ToolResult::success(summary, None))
        })
    }
}
