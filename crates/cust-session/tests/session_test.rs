use cust_session::{RewindMode, SessionMessage, SessionStore};
use std::env;

#[tokio::test]
async fn test_session_create_and_lease() {
    let tmp_dir = env::temp_dir().join("cust_test_sessions");
    let store = SessionStore::new(tmp_dir.clone());

    let (meta, _lease) = store.create_session("test-sess-1", "Test Session").unwrap();
    assert_eq!(meta.id, "test-sess-1");

    // Test second acquire fails with session_already_active
    let store2 = SessionStore::new(tmp_dir);
    let res = store2.create_session("test-sess-1", "Conflict");
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("session_already_active")
    );
}

#[tokio::test]
async fn test_session_append_and_list() {
    let tmp_dir = env::temp_dir().join("cust_test_sessions_list");
    let store = SessionStore::new(tmp_dir);

    let (meta, _lease) = store.create_session("sess-2", "List Test").unwrap();
    store
        .append_message(
            &meta.id,
            &SessionMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            },
        )
        .unwrap();

    let list = store.list_sessions().unwrap();
    assert!(!list.is_empty());
}

#[tokio::test]
async fn test_session_rewind_modes() {
    let tmp_dir = env::temp_dir().join("cust_test_sessions_rewind");
    let store = SessionStore::new(tmp_dir);

    let (meta, _lease) = store.create_session("sess-3", "Rewind Test").unwrap();
    let forked_id = store.rewind_session(&meta.id, 1, RewindMode::Fork).unwrap();
    assert!(forked_id.contains("fork"));
}
