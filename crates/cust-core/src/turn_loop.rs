use crate::event::{EffectId, Event, EventKind, TurnId};
use cust_config_types::Config;
use cust_provider::ProviderClient;
use cust_tools::ToolRegistry;
use cust_tools_api::PermissionRequest;
use futures_util::StreamExt;
use std::pin::Pin;

pub trait ApprovalHandler: Send + Sync {
    fn request_approval(&self, tool: &str, request: &PermissionRequest) -> bool;
}

pub struct DefaultApprovalHandler;
impl ApprovalHandler for DefaultApprovalHandler {
    fn request_approval(&self, _tool: &str, _request: &PermissionRequest) -> bool {
        true
    }
}

pub struct AgentLoop {
    config: Config,
    registry: ToolRegistry,
}

impl AgentLoop {
    pub fn new(config: Config, registry: ToolRegistry) -> Self {
        Self { config, registry }
    }

    pub fn run_turn<'a>(
        &'a self,
        prompt: &'a str,
        approval_handler: &'a dyn ApprovalHandler,
    ) -> Pin<Box<dyn futures_util::Stream<Item = Event> + Send + 'a>> {
        self.run_turn_owned(prompt.to_string(), approval_handler)
    }

    /// Like [`AgentLoop::run_turn`], but takes the prompt by value so the
    /// returned stream does not borrow it. Callers that build prompts in a
    /// loop (the TUI) need this to avoid tying each prompt to the stream.
    pub fn run_turn_owned<'a>(
        &'a self,
        prompt: String,
        approval_handler: &'a dyn ApprovalHandler,
    ) -> Pin<Box<dyn futures_util::Stream<Item = Event> + Send + 'a>> {
        let stream = async_stream::stream! {
            let turn_id: TurnId = 1;
            let mut event_id = 1;
            let effect_counter = 1;

            yield Event {
                id: event_id,
                generation: 1,
                kind: EventKind::TurnStarted { turn: turn_id },
            };
            event_id += 1;

            // System prompt instructions including tool specs
            let mut system_instructions = String::from(
                "You are an AI coding assistant. You have access to tools to read and modify files.\n\
                 When you need to call a tool, respond with a JSON block in the format:\n\
                 ```json\n{\n  \"tool\": \"tool_name\",\n  \"args\": { ... }\n}\n```\n\n\
                 Available tools:\n"
            );

            for spec in self.registry.specs() {
                system_instructions.push_str(&format!(
                    "- {}: {}\n  Parameters: {}\n",
                    spec.name,
                    spec.description,
                    spec.parameters
                ));
            }

            let full_prompt = format!("{system_instructions}\nUser: {prompt}");

            let client = match ProviderClient::from_config(&self.config) {
                Ok(c) => c,
                Err(e) => {
                    yield Event {
                        id: event_id,
                        generation: 1,
                        kind: EventKind::Error {
                            recoverable: false,
                            message: format!("Failed to create provider client: {e}"),
                        },
                    };
                    return;
                }
            };

            let mut text_stream = client.stream_chat(&full_prompt);
            let mut accumulated_text = String::new();

            while let Some(chunk_res) = text_stream.next().await {
                match chunk_res {
                    Ok(text) => {
                        accumulated_text.push_str(&text);
                        yield Event {
                            id: event_id,
                            generation: 1,
                            kind: EventKind::AssistantDelta { text },
                        };
                        event_id += 1;
                    }
                    Err(e) => {
                        yield Event {
                            id: event_id,
                            generation: 1,
                            kind: EventKind::Error {
                                recoverable: true,
                                message: format!("Provider error: {e}"),
                            },
                        };
                        event_id += 1;
                        break;
                    }
                }
            }

            // Check if accumulated text contains tool call JSON block
            if let Some(tool_call) = parse_tool_call(&accumulated_text) {
                let tool_name = tool_call.tool;
                let args = tool_call.args;
                let effect_id: EffectId = effect_counter;

                yield Event {
                    id: event_id,
                    generation: 1,
                    kind: EventKind::ToolCall {
                        effect: effect_id,
                        tool: tool_name.clone(),
                        args: args.clone(),
                    },
                };
                event_id += 1;

                if let Some(tool) = self.registry.get(&tool_name) {
                    let perm_req = tool.permission(&args);
                    let approved = match &perm_req {
                        PermissionRequest::WritePath(_) | PermissionRequest::Execute(_) => {
                            yield Event {
                                id: event_id,
                                generation: 1,
                                kind: EventKind::ApprovalRequested {
                                    effect: effect_id,
                                    tool: tool_name.clone(),
                                    request: perm_req.clone(),
                                },
                            };
                            event_id += 1;
                            approval_handler.request_approval(&tool_name, &perm_req)
                        }
                        _ => true,
                    };

                    if approved {
                        match tool.call(args).await {
                            Ok(result) => {
                                yield Event {
                                    id: event_id,
                                    generation: 1,
                                    kind: EventKind::ToolResult {
                                        effect: effect_id,
                                        ok: result.ok,
                                        summary: result.summary,
                                    },
                                };
                                event_id += 1;
                            }
                            Err(e) => {
                                yield Event {
                                    id: event_id,
                                    generation: 1,
                                    kind: EventKind::ToolResult {
                                        effect: effect_id,
                                        ok: false,
                                        summary: format!("Tool error: {e}"),
                                    },
                                };
                                event_id += 1;
                            }
                        }
                    } else {
                        yield Event {
                            id: event_id,
                            generation: 1,
                            kind: EventKind::ToolResult {
                                effect: effect_id,
                                ok: false,
                                summary: "Permission denied by user".to_string(),
                            },
                        };
                        event_id += 1;
                    }
                } else {
                    yield Event {
                        id: event_id,
                        generation: 1,
                        kind: EventKind::ToolResult {
                            effect: effect_id,
                            ok: false,
                            summary: format!("Unknown tool '{tool_name}'"),
                        },
                    };
                    event_id += 1;
                }
            }

            yield Event {
                id: event_id,
                generation: 1,
                kind: EventKind::TurnEnded {
                    turn: turn_id,
                    reason: "completed".to_string(),
                },
            };
        };

        Box::pin(stream)
    }
}

#[derive(serde::Deserialize)]
struct RawToolCall {
    tool: String,
    args: serde_json::Value,
}

fn parse_tool_call(text: &str) -> Option<RawToolCall> {
    if let Some(start) = text.find("```json") {
        let rest = &text[start + 7..];
        if let Some(end) = rest.find("```") {
            let json_str = rest[..end].trim();
            if let Ok(call) = serde_json::from_str::<RawToolCall>(json_str) {
                return Some(call);
            }
        }
    }
    // Try raw JSON
    if let Ok(call) = serde_json::from_str::<RawToolCall>(text.trim()) {
        return Some(call);
    }
    None
}
