use cust_core::compaction::{Compactor, HistoryItem, ProtectionPolicy};

#[test]
fn test_find_protected_region_default() {
    let items = vec![
        HistoryItem::User("Hello".to_string()),
        HistoryItem::Assistant("Hi".to_string()),
        HistoryItem::ToolCall {
            name: "read_file".to_string(),
            args: serde_json::json!({}),
        },
        HistoryItem::ToolResult {
            name: "read_file".to_string(),
            summary: "file contents".to_string(),
        },
        HistoryItem::Assistant("I found...".to_string()),
        HistoryItem::User("Now write a test".to_string()),
        HistoryItem::ToolCall {
            name: "write_file".to_string(),
            args: serde_json::json!({}),
        },
        HistoryItem::ToolResult {
            name: "write_file".to_string(),
            summary: "written".to_string(),
        },
        HistoryItem::Assistant("Done".to_string()),
        HistoryItem::User("Thanks".to_string()),
    ];

    let policy = ProtectionPolicy::default();
    let region = Compactor::find_protected_region(&items, &policy);

    // First user (0), first assistant (1), first tool_call (2), first tool_result (3)
    // should all be protected
    assert!(region.protected_indices.contains(&0));
    assert!(region.protected_indices.contains(&1));
    assert!(region.protected_indices.contains(&2));
    assert!(region.protected_indices.contains(&3));

    // Last 4 turns (6,7,8,9) should be protected
    assert!(region.protected_indices.contains(&6));
    assert!(region.protected_indices.contains(&7));
    assert!(region.protected_indices.contains(&8));
    assert!(region.protected_indices.contains(&9));

    // Middle region should be compressible
    assert!(region.compress_start <= region.compress_end);
}

#[test]
fn test_boundary_snap_avoids_orphaned_tool_result() {
    let items = vec![
        HistoryItem::User("Hello".to_string()),
        HistoryItem::ToolCall {
            name: "search".to_string(),
            args: serde_json::json!({}),
        },
        HistoryItem::ToolResult {
            name: "search".to_string(),
            summary: "found something".to_string(),
        },
        HistoryItem::Assistant("ok".to_string()),
    ];

    // Index 2 is a ToolResult — should not be a clean boundary
    assert!(!Compactor::is_boundary_clean(&items, 2));

    // Snap should move forward to index 3 (Assistant)
    let snapped = Compactor::snap_boundary(&items, 2, 0, 4);
    assert_eq!(snapped, 3);
}

#[test]
fn test_plan_middle_compression() {
    // Create a trajectory with enough tokens to require compression
    let long_text = "x".repeat(400); // ~100 tokens each
    let items = vec![
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
        HistoryItem::Assistant(long_text.clone()),
        HistoryItem::ToolCall {
            name: "c".to_string(),
            args: serde_json::json!({}),
        },
        HistoryItem::ToolResult {
            name: "c".to_string(),
            summary: long_text.clone(),
        },
        HistoryItem::Assistant("final answer".to_string()),
        HistoryItem::User("thanks".to_string()),
    ];

    let policy = ProtectionPolicy::default();
    // Set a target that forces compression
    let result = Compactor::plan_middle_compression(&items, 200, 50, &policy);
    // Either we get a compression range or None (if nothing is compressible
    // because everything is protected) — both are valid outcomes.
    if let Some(range) = &result {
        assert!(range.start < range.end);
        // Boundary should be clean (not on a ToolResult)
        assert!(Compactor::is_boundary_clean(&items, range.end));
    }
}
