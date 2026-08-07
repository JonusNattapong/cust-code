use cust_exec::{CommandRunner, SandboxProfile};
use cust_tools_api::{Availability, PermissionRequest, Tool, ToolError, ToolResult, ToolSpec};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// Runs shell commands under a sandbox profile chosen at registry-build time.
pub struct BashTool {
    sandbox: SandboxProfile,
}

impl BashTool {
    pub fn new(sandbox: SandboxProfile) -> Self {
        Self { sandbox }
    }

    pub fn sandbox(&self) -> SandboxProfile {
        self.sandbox
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BashArgs {
    pub command: String,
}

impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "bash".to_string(),
            description: "Execute a shell command on the host machine.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command line string to execute"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    fn permission(&self, args: &serde_json::Value) -> PermissionRequest {
        if let Some(cmd) = args.get("command").and_then(|c| c.as_str()) {
            PermissionRequest::Execute(cmd.to_string())
        } else {
            PermissionRequest::None
        }
    }

    fn availability(&self) -> Availability {
        Availability::Direct
    }

    fn call<'a>(
        &'a self,
        args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let bash_args: BashArgs = serde_json::from_value(args)
                .map_err(|e| ToolError::Failed(anyhow::anyhow!("Invalid arguments: {e}")))?;

            let cwd = std::env::current_dir().map_err(|e| {
                ToolError::Failed(anyhow::anyhow!("Failed to get working directory: {e}"))
            })?;

            let res = CommandRunner::run_cmd(&bash_args.command, &cwd, self.sandbox)
                .await
                .map_err(ToolError::Failed)?;

            let output = format!(
                "Exit code: {}\nStdout:\n{}\nStderr:\n{}",
                res.exit_code, res.stdout, res.stderr
            );

            let summary = format!(
                "Executed command '{}' (exit code {})",
                bash_args.command, res.exit_code
            );
            Ok(ToolResult::success(
                summary,
                Some(serde_json::json!(output)),
            ))
        })
    }
}
