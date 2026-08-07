use cust_tools_api::{Availability, PermissionRequest, Tool, ToolError, ToolResult, ToolSpec};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub struct ReadFileTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadFileArgs {
    pub path: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
}

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_file".to_string(),
            description: "Read text contents of a file from the filesystem.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "1-indexed line number to start reading from"
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "1-indexed line number to stop reading at (inclusive)"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    fn permission(&self, args: &serde_json::Value) -> PermissionRequest {
        if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
            PermissionRequest::ReadPath(PathBuf::from(path))
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
            let read_args: ReadFileArgs = serde_json::from_value(args)
                .map_err(|e| ToolError::Failed(anyhow::anyhow!("Invalid arguments: {e}")))?;

            let path = PathBuf::from(&read_args.path);
            let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
                ToolError::Failed(anyhow::anyhow!(
                    "Failed to read file {}: {e}",
                    path.display()
                ))
            })?;

            let lines: Vec<&str> = content.lines().collect();
            let start = read_args.start_line.unwrap_or(1).saturating_sub(1);
            let end = read_args.end_line.unwrap_or(lines.len()).min(lines.len());

            if start >= lines.len() {
                return Ok(ToolResult::success(
                    format!("File {} has {} lines", path.display(), lines.len()),
                    Some(serde_json::json!("")),
                ));
            }

            let slice = &lines[start..end];
            let mut formatted = String::new();
            for (idx, line) in slice.iter().enumerate() {
                formatted.push_str(&format!("{:4}: {}\n", start + idx + 1, line));
            }

            let summary = format!("Read {} lines from {}", slice.len(), path.display());
            Ok(ToolResult::success(
                summary,
                Some(serde_json::json!(formatted)),
            ))
        })
    }
}
