use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyringError {
    #[error("Keyring entry not found for service '{service}', account '{account}'")]
    NotFound { service: String, account: String },
    #[error("Keyring storage error: {0}")]
    StorageError(String),
}

pub trait KeyringStore: Send + Sync {
    fn load(&self, service: &str, account: &str) -> Result<Option<String>, KeyringError>;
    fn save(&self, service: &str, account: &str, value: &str) -> Result<(), KeyringError>;
    fn delete(&self, service: &str, account: &str) -> Result<bool, KeyringError>;
}

#[derive(Default, Clone)]
pub struct MockKeyringStore {
    storage: Arc<Mutex<HashMap<String, String>>>,
}

impl MockKeyringStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(service: &str, account: &str) -> String {
        format!("{service}:{account}")
    }
}

impl KeyringStore for MockKeyringStore {
    fn load(&self, service: &str, account: &str) -> Result<Option<String>, KeyringError> {
        let key = Self::key(service, account);
        let map = self.storage.lock().unwrap();
        Ok(map.get(&key).cloned())
    }

    fn save(&self, service: &str, account: &str, value: &str) -> Result<(), KeyringError> {
        let key = Self::key(service, account);
        let mut map = self.storage.lock().unwrap();
        map.insert(key, value.to_string());
        Ok(())
    }

    fn delete(&self, service: &str, account: &str) -> Result<bool, KeyringError> {
        let key = Self::key(service, account);
        let mut map = self.storage.lock().unwrap();
        Ok(map.remove(&key).is_some())
    }
}
