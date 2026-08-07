use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const MAX_SUBAGENT_DEPTH: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryMode {
    Steer,
    FollowUp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub sender_id: String,
    pub recipient_id: String,
    pub content: String,
    pub mode: DeliveryMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskWaitMode {
    Any,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHandle {
    pub task_id: String,
    pub parent_depth: usize,
    pub status: String,
    pub output: Option<String>,
}

// ---------------------------------------------------------------------------
// Programmatic Subagent REPL (inspired by prime-agent)
//
// prime-agent spawns subagents inside a persistent REPL via `rlm("task", ...)`
// which returns structured data directly into variables. We model this with
// `ReplInvocation` — a typed request/response pair that the code-mode engine
// can emit and the subagent manager can fulfill.
// ---------------------------------------------------------------------------

/// A programmatic subagent invocation that returns structured JSON data,
/// analogous to prime-agent's `rlm("task", ...)` REPL function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplInvocation {
    /// Unique invocation ID (monotonically increasing).
    pub invocation_id: u64,
    /// The prompt / task description for the subagent.
    pub prompt: String,
    /// Optional structured input data (passed as JSON context).
    pub input_data: Option<serde_json::Value>,
    /// Structured output returned by the subagent (populated after completion).
    pub result: Option<serde_json::Value>,
    /// Error message if the invocation failed.
    pub error: Option<String>,
}

#[derive(Default)]
pub struct SubagentManager {
    tasks: Arc<Mutex<HashMap<String, TaskHandle>>>,
    messages: Arc<Mutex<Vec<AgentMessage>>>,
    counter: Arc<Mutex<usize>>,
    repl_invocations: Arc<Mutex<Vec<ReplInvocation>>>,
    repl_counter: Arc<Mutex<u64>>,
}

impl SubagentManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(Vec::new())),
            counter: Arc::new(Mutex::new(1)),
            repl_invocations: Arc::new(Mutex::new(Vec::new())),
            repl_counter: Arc::new(Mutex::new(1)),
        }
    }

    pub fn send_message(&self, msg: AgentMessage) {
        self.messages.lock().unwrap().push(msg);
    }

    pub fn drain_messages_for(&self, recipient_id: &str, mode: DeliveryMode) -> Vec<AgentMessage> {
        let mut msgs = self.messages.lock().unwrap();
        let mut matched = Vec::new();
        msgs.retain(|m| {
            if m.recipient_id == recipient_id && m.mode == mode {
                matched.push(m.clone());
                false
            } else {
                true
            }
        });
        matched
    }

    pub fn spawn_task(&self, current_depth: usize, prompt: &str) -> Result<String, anyhow::Error> {
        if current_depth >= MAX_SUBAGENT_DEPTH {
            return Err(anyhow::anyhow!(
                "Subagent depth limit exceeded (max depth {MAX_SUBAGENT_DEPTH})"
            ));
        }

        let mut cnt = self.counter.lock().unwrap();
        let task_id = format!("subagent-task-{cnt}");
        *cnt += 1;

        let handle = TaskHandle {
            task_id: task_id.clone(),
            parent_depth: current_depth + 1,
            status: "Running".to_string(),
            output: Some(format!("Subagent completed prompt: '{prompt}'")),
        };

        self.tasks.lock().unwrap().insert(task_id.clone(), handle);
        Ok(task_id)
    }

    pub fn get_output(&self, task_id: &str) -> Option<String> {
        self.tasks
            .lock()
            .unwrap()
            .get(task_id)
            .and_then(|t| t.output.clone())
    }

    pub fn wait_tasks(&self, task_ids: &[String], _mode: TaskWaitMode) -> Vec<String> {
        let tasks = self.tasks.lock().unwrap();
        task_ids
            .iter()
            .filter_map(|id| tasks.get(id).and_then(|t| t.output.clone()))
            .collect()
    }

    pub fn kill_task(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(t) = tasks.get_mut(task_id) {
            t.status = "Killed".to_string();
            true
        } else {
            false
        }
    }

    // -----------------------------------------------------------------------
    // Programmatic REPL invocation API
    // -----------------------------------------------------------------------

    /// Create a programmatic subagent invocation (analogous to `rlm("task")`).
    /// Returns the invocation ID so the caller can later retrieve the result.
    pub fn repl_invoke(&self, prompt: &str, input_data: Option<serde_json::Value>) -> u64 {
        let mut counter = self.repl_counter.lock().unwrap();
        let id = *counter;
        *counter += 1;

        let invocation = ReplInvocation {
            invocation_id: id,
            prompt: prompt.to_string(),
            input_data,
            result: None,
            error: None,
        };

        self.repl_invocations.lock().unwrap().push(invocation);
        id
    }

    /// Complete a REPL invocation with a structured result.
    pub fn repl_complete(&self, invocation_id: u64, result: serde_json::Value) -> bool {
        let mut invocations = self.repl_invocations.lock().unwrap();
        if let Some(inv) = invocations
            .iter_mut()
            .find(|i| i.invocation_id == invocation_id)
        {
            inv.result = Some(result);
            true
        } else {
            false
        }
    }

    /// Mark a REPL invocation as failed.
    pub fn repl_fail(&self, invocation_id: u64, error: &str) -> bool {
        let mut invocations = self.repl_invocations.lock().unwrap();
        if let Some(inv) = invocations
            .iter_mut()
            .find(|i| i.invocation_id == invocation_id)
        {
            inv.error = Some(error.to_string());
            true
        } else {
            false
        }
    }

    /// Retrieve the result of a completed REPL invocation.
    pub fn repl_get_result(&self, invocation_id: u64) -> Option<serde_json::Value> {
        let invocations = self.repl_invocations.lock().unwrap();
        invocations
            .iter()
            .find(|i| i.invocation_id == invocation_id)
            .and_then(|i| i.result.clone())
    }

    /// List all pending (not yet completed) REPL invocations.
    pub fn repl_pending(&self) -> Vec<ReplInvocation> {
        let invocations = self.repl_invocations.lock().unwrap();
        invocations
            .iter()
            .filter(|i| i.result.is_none() && i.error.is_none())
            .cloned()
            .collect()
    }
}
