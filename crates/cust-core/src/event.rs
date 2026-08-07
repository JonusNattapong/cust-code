use cust_tools_api::PermissionRequest;
use serde::{Deserialize, Serialize};

pub type EventId = u64;
pub type TurnId = u64;
pub type EffectId = u64;
pub type Generation = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub generation: Generation,
    pub kind: EventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub enum EventKind {
    TurnStarted {
        turn: TurnId,
    },
    AssistantDelta {
        text: String,
    },
    ReasoningDelta {
        text: String,
    },
    ToolCall {
        effect: EffectId,
        tool: String,
        args: serde_json::Value,
    },
    ToolStream {
        effect: EffectId,
        chunk: String,
    },
    ToolResult {
        effect: EffectId,
        ok: bool,
        summary: String,
    },
    ApprovalRequested {
        effect: EffectId,
        tool: String,
        request: PermissionRequest,
    },
    TurnEnded {
        turn: TurnId,
        reason: String,
    },
    Error {
        recoverable: bool,
        message: String,
    },
}
