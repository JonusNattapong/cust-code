use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyOutput {
    pub exit_code: i32,
    pub raw_output: String,
    pub clean_output: String,
}

pub struct PtyRunner;

impl PtyRunner {
    pub fn run_interactive(cmd: &str, args: &[&str]) -> Result<PtyOutput, anyhow::Error> {
        let mut child_cmd = Command::new(cmd);
        child_cmd.args(args);

        let output = child_cmd.output()?;
        let raw = String::from_utf8_lossy(&output.stdout).to_string();
        let clean = Self::strip_ansi(&raw);

        Ok(PtyOutput {
            exit_code: output.status.code().unwrap_or(-1),
            raw_output: raw,
            clean_output: clean,
        })
    }

    pub fn strip_ansi(text: &str) -> String {
        // Strip basic ANSI escape sequences (ESC[...]m)
        let mut clean = String::with_capacity(text.len());
        let mut in_escape = false;

        for ch in text.chars() {
            if ch == '\x1B' {
                in_escape = true;
            } else if in_escape {
                if ch.is_ascii_alphabetic() {
                    in_escape = false;
                }
            } else {
                clean.push(ch);
            }
        }
        clean
    }
}
