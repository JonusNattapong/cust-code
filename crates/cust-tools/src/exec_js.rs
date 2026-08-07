use cust_codemode::{CodeEvaluator, HostBridge};
use cust_tools_api::{Availability, PermissionRequest, Tool, ToolError, ToolResult, ToolSpec};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub struct ExecJsTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecJsArgs {
    pub code: String,
}

impl Tool for ExecJsTool {
    fn name(&self) -> &str {
        "exec"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "exec".to_string(),
            description:
                "Execute a JavaScript code script in a zero-capability QuickJS guest environment."
                    .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "JavaScript code snippet to execute"
                    }
                },
                "required": ["code"]
            }),
        }
    }

    fn permission(&self, _args: &serde_json::Value) -> PermissionRequest {
        PermissionRequest::None
    }

    fn availability(&self) -> Availability {
        Availability::Direct
    }

    fn call<'a>(
        &'a self,
        args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let exec_args: ExecJsArgs = serde_json::from_value(args)
                .map_err(|e| ToolError::Failed(anyhow::anyhow!("Invalid arguments: {e}")))?;

            let default_tools = crate::registry::ToolRegistry::with_default_tools();
            let mut map = std::collections::HashMap::new();
            for spec in default_tools.specs() {
                if let Some(tool) = default_tools.get(&spec.name) {
                    map.insert(spec.name.clone(), tool);
                }
            }

            let bridge = HostBridge::new(map);
            let evaluator = CodeEvaluator::new(bridge);

            let res = evaluator
                .eval_script(&exec_args.code)
                .await
                .map_err(ToolError::Failed)?;

            let summary = "Executed JavaScript code mode script successfully".to_string();
            Ok(ToolResult::success(summary, Some(serde_json::json!(res))))
        })
    }
}
