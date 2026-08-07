use cust_proto::{EventCursor, IdempotencyJournal};

#[test]
fn test_idempotency_journal() {
    let mut journal = IdempotencyJournal::new();

    // First dispatch succeeds
    assert!(journal.record_before_dispatch("client1", "cmd1").is_ok());

    // Second dispatch with same client_id + command_id fails
    assert!(journal.record_before_dispatch("client1", "cmd1").is_err());

    // Mark completed
    journal.mark_completed("client1", "cmd1", serde_json::json!({ "status": "ok" }));

    // Additional dispatch fails as already executed
    assert!(journal.record_before_dispatch("client1", "cmd1").is_err());
}

#[test]
fn test_event_cursor() {
    let mut cursor = EventCursor::new(1, 10);
    cursor.advance();
    assert_eq!(cursor.sequence, 11);
}
