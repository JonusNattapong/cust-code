use crate::compaction::HistoryItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineMemory {
    pub key: String,
    pub value: String,
    pub previous_value: Option<String>,
}

pub struct RefineEngine {
    memories: Vec<RefineMemory>,
}

impl Default for RefineEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RefineEngine {
    pub fn new() -> Self {
        Self {
            memories: Vec::new(),
        }
    }

    pub fn refine_trajectory(&mut self, items: &[HistoryItem]) -> String {
        let user_count = items
            .iter()
            .filter(|i| matches!(i, HistoryItem::User(_)))
            .count();
        let key = format!("trajectory_summary_{user_count}");
        let val = format!("Observed trajectory with {user_count} user turns.");

        let prev = self
            .memories
            .iter()
            .find(|m| m.key == key)
            .map(|m| m.value.clone());

        self.memories.push(RefineMemory {
            key,
            value: val.clone(),
            previous_value: prev,
        });

        val
    }

    pub fn memories(&self) -> &[RefineMemory] {
        &self.memories
    }
}
