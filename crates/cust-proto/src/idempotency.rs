use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandStatus {
    Pending,
    Completed(serde_json::Value),
    Uncertain(String),
}

#[derive(Default)]
pub struct IdempotencyJournal {
    records: HashMap<String, CommandStatus>,
}

impl IdempotencyJournal {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    pub fn key(client_id: &str, command_id: &str) -> String {
        format!("{client_id}:{command_id}")
    }

    pub fn record_before_dispatch(
        &mut self,
        client_id: &str,
        command_id: &str,
    ) -> Result<(), anyhow::Error> {
        let k = Self::key(client_id, command_id);
        if let Some(existing) = self.records.get(&k) {
            match existing {
                CommandStatus::Pending => {
                    Err(anyhow::anyhow!("Command is currently pending execution"))
                }
                CommandStatus::Completed(_) => {
                    Err(anyhow::anyhow!("Command has already been executed"))
                }
                CommandStatus::Uncertain(msg) => Err(anyhow::anyhow!(
                    "Command execution is uncertain: {msg} (will not replay)"
                )),
            }
        } else {
            self.records.insert(k, CommandStatus::Pending);
            Ok(())
        }
    }

    pub fn mark_completed(&mut self, client_id: &str, command_id: &str, result: serde_json::Value) {
        let k = Self::key(client_id, command_id);
        self.records.insert(k, CommandStatus::Completed(result));
    }

    pub fn mark_uncertain(&mut self, client_id: &str, command_id: &str, reason: &str) {
        let k = Self::key(client_id, command_id);
        self.records
            .insert(k, CommandStatus::Uncertain(reason.to_string()));
    }
}
