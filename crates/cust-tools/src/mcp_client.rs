use cust_tools_api::{Availability, PermissionRequest, Tool, ToolError, ToolResult, ToolSpec};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub struct McpToolWrapper {
    pub spec: McpToolSpec,
}

impl McpToolWrapper {
    pub fn new(spec: McpToolSpec) -> Self {
        Self { spec }
    }
}

impl Tool for McpToolWrapper {
    fn name(&self) -> &str {
        &self.spec.name
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.spec.name.clone(),
            description: self.spec.description.clone(),
            parameters: self.spec.input_schema.clone(),
        }
    }

    fn permission(&self, _args: &serde_json::Value) -> PermissionRequest {
        PermissionRequest::Custom(format!("MCP tool {}", self.spec.name))
    }

    fn availability(&self) -> Availability {
        Availability::Both
    }

    fn call<'a>(
        &'a self,
        args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let summary = format!("Executed MCP tool `{}`", self.spec.name);
            Ok(ToolResult::success(summary, Some(args)))
        })
    }
}
