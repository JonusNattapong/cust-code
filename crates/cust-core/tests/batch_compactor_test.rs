use cust_core::{BatchCompactor, BatchTask, HistoryItem, ProtectionPolicy};

#[tokio::test]
async fn test_batch_compactor_parallel_execution() {
    let long_text = "x".repeat(300);
    let history = vec![
        HistoryItem::User("task".to_string()),
        HistoryItem::Assistant("ok".to_string()),
        HistoryItem::ToolCall {
            name: "a".to_string(),
            args: serde_json::json!({}),
        },
        HistoryItem::ToolResult {
            name: "a".to_string(),
            summary: long_text.clone(),
        },
        HistoryItem::Assistant(long_text.clone()),
        HistoryItem::ToolCall {
            name: "b".to_string(),
            args: serde_json::json!({}),
        },
        HistoryItem::ToolResult {
            name: "b".to_string(),
            summary: long_text.clone(),
        },
        HistoryItem::Assistant("done".to_string()),
        HistoryItem::User("thanks".to_string()),
    ];

    let tasks = vec![
        BatchTask {
            session_id: "s1".to_string(),
            history: history.clone(),
            target_tokens: 150,
        },
        BatchTask {
            session_id: "s2".to_string(),
            history: history.clone(),
            target_tokens: 150,
        },
    ];

    let results = BatchCompactor::compact_batch(tasks, ProtectionPolicy::default()).await;
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].session_id, "s1");
    assert_eq!(results[1].session_id, "s2");
}
