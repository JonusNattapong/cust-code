use cust_core::SubagentManager;

#[test]
fn test_repl_invoke_and_complete() {
    let mgr = SubagentManager::new();

    // Invoke a programmatic subagent
    let inv_id = mgr.repl_invoke(
        "Analyze code quality",
        Some(serde_json::json!({ "file": "src/main.rs" })),
    );
    assert_eq!(inv_id, 1);

    // Should appear as pending
    let pending = mgr.repl_pending();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].prompt, "Analyze code quality");

    // Complete with structured result
    let result_data = serde_json::json!({
        "score": 95,
        "issues": [],
        "recommendation": "No changes needed"
    });
    assert!(mgr.repl_complete(inv_id, result_data.clone()));

    // Result should be available
    let result = mgr.repl_get_result(inv_id);
    assert!(result.is_some());
    assert_eq!(result.unwrap()["score"], 95);

    // Should no longer be pending
    assert!(mgr.repl_pending().is_empty());
}

#[test]
fn test_repl_fail() {
    let mgr = SubagentManager::new();
    let inv_id = mgr.repl_invoke("Bad task", None);
    assert!(mgr.repl_fail(inv_id, "Depth limit exceeded"));

    // Should no longer be pending
    assert!(mgr.repl_pending().is_empty());
    // Result should be None (failed, not completed)
    assert!(mgr.repl_get_result(inv_id).is_none());
}

#[test]
fn test_repl_multiple_invocations() {
    let mgr = SubagentManager::new();

    let id1 = mgr.repl_invoke("Task A", None);
    let id2 = mgr.repl_invoke("Task B", None);
    let id3 = mgr.repl_invoke("Task C", None);

    assert_eq!(mgr.repl_pending().len(), 3);

    mgr.repl_complete(id2, serde_json::json!("B done"));
    assert_eq!(mgr.repl_pending().len(), 2);

    mgr.repl_complete(id1, serde_json::json!("A done"));
    mgr.repl_fail(id3, "cancelled");
    assert!(mgr.repl_pending().is_empty());
}
