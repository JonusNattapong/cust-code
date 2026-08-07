use crate::cmd_parser::ShellPlan;
use crate::sandbox::SandboxProfile;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct CommandRunner;

impl CommandRunner {
    pub async fn run_cmd(
        cmd: &str,
        cwd: &Path,
        profile: SandboxProfile,
    ) -> Result<ExecutionResult, anyhow::Error> {
        let plan = ShellPlan::parse(cmd);

        // Check sandbox policy against parsed plan target paths
        for target in &plan.target_paths {
            profile.check_permission(cwd, false, Some(target))?;
        }

        let mut command = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", cmd]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", cmd]);
            c
        };

        command.current_dir(cwd);
        let output = command
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to execute process: {e}"))?;

        let max_len = 10 * 1024 * 1024; // 10 MiB limit
        let stdout_raw = String::from_utf8_lossy(&output.stdout);
        let stderr_raw = String::from_utf8_lossy(&output.stderr);

        let stdout = if stdout_raw.len() > max_len {
            format!("{}... [truncated]", &stdout_raw[..max_len])
        } else {
            stdout_raw.to_string()
        };

        let stderr = if stderr_raw.len() > max_len {
            format!("{}... [truncated]", &stderr_raw[..max_len])
        } else {
            stderr_raw.to_string()
        };

        let exit_code = output.status.code().unwrap_or(-1);

        Ok(ExecutionResult {
            exit_code,
            stdout,
            stderr,
        })
    }
}
