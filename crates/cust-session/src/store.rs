use crate::lease::SessionLease;
use crate::rewind::RewindMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
}

pub struct SessionStore {
    base_dir: PathBuf,
}

impl SessionStore {
    pub fn new(base_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&base_dir);
        Self { base_dir }
    }

    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cust")
            .join("sessions")
    }

    pub fn create_session(
        &self,
        id: &str,
        title: &str,
    ) -> Result<(SessionMetadata, SessionLease), anyhow::Error> {
        let lock_path = self.base_dir.join(format!("{id}.lock"));
        let lease = SessionLease::acquire(lock_path)?;

        let meta = SessionMetadata {
            id: id.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            title: title.to_string(),
        };

        let meta_path = self.base_dir.join(format!("{id}.meta.json"));
        let meta_json = serde_json::to_string_pretty(&meta)?;
        std::fs::write(meta_path, meta_json)?;

        Ok((meta, lease))
    }

    pub fn append_message(&self, id: &str, msg: &SessionMessage) -> Result<(), anyhow::Error> {
        let jsonl_path = self.base_dir.join(format!("{id}.jsonl"));
        let line = serde_json::to_string(msg)? + "\n";
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(jsonl_path)?;
        file.write_all(line.as_bytes())?;
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionMetadata>, anyhow::Error> {
        let mut list = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json")
                    && path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("")
                        .ends_with(".meta.json")
                {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(meta) = serde_json::from_str::<SessionMetadata>(&content) {
                            list.push(meta);
                        }
                    }
                }
            }
        }
        list.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(list)
    }

    pub fn load_session(
        &self,
        id: &str,
    ) -> Result<(SessionMetadata, Vec<SessionMessage>), anyhow::Error> {
        let meta_path = self.base_dir.join(format!("{id}.meta.json"));
        let meta_json = std::fs::read_to_string(&meta_path)
            .map_err(|_| anyhow::anyhow!("no saved session with id '{id}'"))?;
        let meta: SessionMetadata = serde_json::from_str(&meta_json)?;

        let jsonl_path = self.base_dir.join(format!("{id}.jsonl"));
        let mut messages = Vec::new();
        if let Ok(content) = std::fs::read_to_string(&jsonl_path) {
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                messages.push(serde_json::from_str::<SessionMessage>(line)?);
            }
        }

        Ok((meta, messages))
    }

    pub fn rewind_session(
        &self,
        id: &str,
        _target_turn: usize,
        mode: RewindMode,
    ) -> Result<String, anyhow::Error> {
        match mode {
            RewindMode::Fork => {
                let new_id = format!(
                    "{id}-fork-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                );
                self.create_session(&new_id, &format!("Fork of {id}"))?;
                Ok(new_id)
            }
            RewindMode::InPlace => Ok(id.to_string()),
        }
    }
}
