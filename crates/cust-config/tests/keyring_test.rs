use cust_config::{KeyringStore, MockKeyringStore};

#[test]
fn test_mock_keyring_store_crud() {
    let store = MockKeyringStore::new();

    // 1. Initial state is empty
    assert_eq!(store.load("cust", "openai_key").unwrap(), None);

    // 2. Save credential
    store.save("cust", "openai_key", "sk-test-12345").unwrap();
    assert_eq!(
        store.load("cust", "openai_key").unwrap(),
        Some("sk-test-12345".to_string())
    );

    // 3. Delete credential
    assert!(store.delete("cust", "openai_key").unwrap());
    assert_eq!(store.load("cust", "openai_key").unwrap(), None);
}
