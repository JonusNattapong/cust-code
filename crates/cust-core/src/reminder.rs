use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReminderKind {
    CurrentTime(String),
    TokenBudget {
        tokens_left: usize,
        context_window: usize,
    },
    SubagentNotification {
        task_id: String,
        output: String,
    },
    PermissionInstructions(String),
}

pub struct ReminderRegistry {
    reminders: Vec<ReminderKind>,
}

impl Default for ReminderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ReminderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            reminders: Vec::new(),
        };
        let now_str = chrono_string();
        registry.add(ReminderKind::CurrentTime(now_str));
        registry
    }

    pub fn add(&mut self, reminder: ReminderKind) {
        self.reminders.push(reminder);
    }

    pub fn render_system_reminder_block(&self) -> String {
        if self.reminders.is_empty() {
            return String::new();
        }

        let mut out = String::from("### System Reminders\n");
        for r in &self.reminders {
            match r {
                ReminderKind::CurrentTime(t) => {
                    out.push_str(&format!("- Current local time: {t}\n"));
                }
                ReminderKind::TokenBudget {
                    tokens_left,
                    context_window,
                } => {
                    out.push_str(&format!(
                        "- Context budget: {tokens_left} / {context_window} tokens remaining.\n"
                    ));
                }
                ReminderKind::SubagentNotification { task_id, output } => {
                    out.push_str(&format!(
                        "- Subagent task `{task_id}` finished with output: {output}\n"
                    ));
                }
                ReminderKind::PermissionInstructions(instr) => {
                    out.push_str(&format!("- Security policy: {instr}\n"));
                }
            }
        }
        out
    }
}

fn chrono_string() -> String {
    let now = std::time::SystemTime::now();
    let since_epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("Unix timestamp: {}", since_epoch.as_secs())
}
