use cust_tools_api::{Availability, PermissionRequest, Tool, ToolError, ToolResult, ToolSpec};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub struct ListDirTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListDirArgs {
    pub path: Option<String>,
}

impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_dir".to_string(),
            description: "List contents of a directory on the filesystem.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the directory to list (defaults to current directory '.')"
                    }
                }
            }),
        }
    }

    fn permission(&self, args: &serde_json::Value) -> PermissionRequest {
        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or(".");
        PermissionRequest::ReadPath(PathBuf::from(path))
    }

    fn availability(&self) -> Availability {
        Availability::Both
    }

    fn call<'a>(
        &'a self,
        args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let list_args: ListDirArgs =
                serde_json::from_value(args).unwrap_or(ListDirArgs { path: None });
            let dir_path = PathBuf::from(list_args.path.unwrap_or_else(|| ".".to_string()));

            let mut entries = tokio::fs::read_dir(&dir_path).await.map_err(|e| {
                ToolError::Failed(anyhow::anyhow!(
                    "Failed to read directory {}: {e}",
                    dir_path.display()
                ))
            })?;

            let mut items = Vec::new();
            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| ToolError::Failed(anyhow::Error::from(e)))?
            {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let file_type = entry.file_type().await.ok();
                let is_dir = file_type.map(|t| t.is_dir()).unwrap_or(false);
                if is_dir {
                    items.push(format!("{file_name}/"));
                } else {
                    items.push(file_name);
                }
            }

            items.sort();
            let summary = format!("Listed {} entries in {}", items.len(), dir_path.display());
            let data = items.join("\n");
            Ok(ToolResult::success(summary, Some(serde_json::json!(data))))
        })
    }
}
