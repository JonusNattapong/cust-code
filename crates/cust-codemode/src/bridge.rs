use cust_tools_api::{Availability, Tool, ToolError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub enum HostRequest {
    CallTool {
        name: String,
        args: serde_json::Value,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HostReply {
    ToolResult {
        ok: bool,
        summary: String,
        data: Option<serde_json::Value>,
    },
    Error(String),
}

pub struct HostBridge {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl HostBridge {
    pub fn new(tools: HashMap<String, Arc<dyn Tool>>) -> Self {
        // Filter out tools that are not available in code mode
        let filtered_tools = tools
            .into_iter()
            .filter(|(_, tool)| match tool.availability() {
                Availability::CodeMode | Availability::Both => true,
                Availability::Direct => false,
            })
            .collect();

        Self {
            tools: filtered_tools,
        }
    }

    pub fn available_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub async fn handle_request(&self, req: HostRequest) -> HostReply {
        match req {
            HostRequest::CallTool { name, args } => {
                if let Some(tool) = self.tools.get(&name) {
                    match tool.call(args).await {
                        Ok(res) => HostReply::ToolResult {
                            ok: res.ok,
                            summary: res.summary,
                            data: res.data,
                        },
                        Err(ToolError::Denied(reason)) => {
                            HostReply::Error(format!("Permission denied: {reason}"))
                        }
                        Err(ToolError::Failed(err)) => {
                            HostReply::Error(format!("Tool failed: {err}"))
                        }
                    }
                } else {
                    HostReply::Error(format!("Tool '{name}' is not available in code mode"))
                }
            }
        }
    }
}
