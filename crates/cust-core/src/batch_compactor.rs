use crate::compaction::{Compactor, HistoryItem, ProtectionPolicy};
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct BatchTask {
    pub session_id: String,
    pub history: Vec<HistoryItem>,
    pub target_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct BatchResult {
    pub session_id: String,
    pub compression_range: Option<Range<usize>>,
}

pub struct BatchCompactor;

impl BatchCompactor {
    pub async fn compact_batch(
        tasks: Vec<BatchTask>,
        policy: ProtectionPolicy,
    ) -> Vec<BatchResult> {
        let mut handles = Vec::new();

        for task in tasks {
            let pol = policy.clone();
            let handle = tokio::spawn(async move {
                let range = Compactor::plan_middle_compression(
                    &task.history,
                    task.target_tokens,
                    50,
                    &pol,
                );
                BatchResult {
                    session_id: task.session_id,
                    compression_range: range,
                }
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(res) = handle.await {
                results.push(res);
            }
        }
        results
    }
}
