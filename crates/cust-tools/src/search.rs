use cust_tools_api::{Availability, PermissionRequest, Tool, ToolError, ToolResult, ToolSpec};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub struct SearchTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchArgs {
    pub query: String,
    pub path: Option<String>,
}

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "search".to_string(),
            description: "Search for text matching a query string in files within a directory."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Text query string to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Root directory path to search in (defaults to '.')"
                    }
                },
                "required": ["query"]
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
            let search_args: SearchArgs = serde_json::from_value(args)
                .map_err(|e| ToolError::Failed(anyhow::anyhow!("Invalid arguments: {e}")))?;

            let root_path = PathBuf::from(search_args.path.unwrap_or_else(|| ".".to_string()));
            let query = search_args.query.to_lowercase();
            let mut matches = Vec::new();

            let mut stack = vec![root_path.clone()];
            while let Some(dir) = stack.pop() {
                if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();
                        let file_name = entry.file_name().to_string_lossy().to_string();

                        // Skip hidden dirs / target / git
                        if file_name.starts_with('.')
                            || file_name == "target"
                            || file_name == "node_modules"
                        {
                            continue;
                        }

                        if let Ok(file_type) = entry.file_type().await {
                            if file_type.is_dir() {
                                stack.push(path);
                            } else if file_type.is_file() {
                                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                                    for (line_idx, line) in content.lines().enumerate() {
                                        if line.to_lowercase().contains(&query) {
                                            matches.push(format!(
                                                "{}:{}: {}",
                                                path.display(),
                                                line_idx + 1,
                                                line.trim()
                                            ));
                                            if matches.len() >= 50 {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if matches.len() >= 50 {
                            break;
                        }
                    }
                }
            }

            let summary = format!(
                "Found {} matching lines for '{}' under {}",
                matches.len(),
                query,
                root_path.display()
            );
            let data = matches.join("\n");
            Ok(ToolResult::success(summary, Some(serde_json::json!(data))))
        })
    }
}
