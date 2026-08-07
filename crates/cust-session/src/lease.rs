use std::path::PathBuf;

#[derive(Debug)]
pub struct SessionLease {
    lock_path: PathBuf,
}

impl SessionLease {
    pub fn acquire(lock_path: PathBuf) -> Result<Self, anyhow::Error> {
        if lock_path.exists() {
            let pid_str = std::fs::read_to_string(&lock_path).unwrap_or_default();
            let pid = pid_str.trim();
            return Err(anyhow::anyhow!(
                "session_already_active: Session is already in use by owner process PID {pid}"
            ));
        }

        let pid = std::process::id();
        if let Some(parent) = lock_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&lock_path, pid.to_string())?;

        Ok(Self { lock_path })
    }
}

impl Drop for SessionLease {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}
