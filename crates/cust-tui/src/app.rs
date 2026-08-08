use crate::banner::BannerInfo;
use crate::permission::{PermissionMode, SharedPermissionMode};
use crate::statusline::{StatusLineConfig, StatusLineUpdate};
use cust_core::{Event, EventKind, SlashRegistry};

/// What the TUI should do with a submitted line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashOutcome {
    /// Handled locally; nothing goes to the agent.
    Consumed,
    /// Send this prompt to the agent.
    Prompt(String),
    /// Leave the TUI.
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Agent,
    TerminalShell,
}

pub struct TuiState {
    pub status: String,
    pub active_model: String,
    pub active_provider: String,
    pub assistant_text: String,
    pub logs: Vec<String>,
    pub view_mode: ViewMode,
    pub current_tokens: usize,
    pub max_context_tokens: usize,
    pub input_buffer: String,
    pub slash_suggestions: Vec<String>,
    /// Welcome banner shown until the first turn starts.
    pub banner: BannerInfo,
    pub show_banner: bool,
    /// Live permission mode, shared with the agent's approval callback.
    pub permission_mode: SharedPermissionMode,
    /// Which status line segments are shown; configured by `/statusline`.
    pub statusline: StatusLineConfig,
    /// Workspace name shown in the status line.
    pub workspace: String,
    /// Current git branch, when the workspace is a repository.
    pub git_branch: Option<String>,
    slash_registry: SlashRegistry,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            status: "Idle".to_string(),
            active_model: "gpt-4o".to_string(),
            active_provider: "openai".to_string(),
            assistant_text: String::new(),
            logs: Vec::new(),
            view_mode: ViewMode::default(),
            current_tokens: 0,
            max_context_tokens: 128_000,
            input_buffer: String::new(),
            slash_suggestions: Vec::new(),
            banner: BannerInfo {
                model: "gpt-4o".to_string(),
                provider: "openai".to_string(),
                ..BannerInfo::default()
            },
            show_banner: true,
            permission_mode: SharedPermissionMode::new(PermissionMode::default()),
            statusline: StatusLineConfig::default(),
            workspace: String::new(),
            git_branch: None,
            slash_registry: SlashRegistry::new(),
        }
    }
}

impl TuiState {
    pub fn toggle_view_mode(&mut self) -> ViewMode {
        self.view_mode = match self.view_mode {
            ViewMode::Agent => ViewMode::TerminalShell,
            ViewMode::TerminalShell => ViewMode::Agent,
        };
        self.view_mode
    }

    /// Handle a slash command locally.
    ///
    /// Commands the TUI owns (display, permissions, quitting) are applied here;
    /// everything else becomes a prompt for the agent.
    pub fn handle_slash(&mut self, input: &str) -> SlashOutcome {
        let Some((name, args)) = self.slash_registry.parse_input(input) else {
            return SlashOutcome::Prompt(input.to_string());
        };
        let args = args.to_string();

        match name {
            "statusline" => {
                let message = match self.statusline.apply(&args) {
                    StatusLineUpdate::Changed(msg) | StatusLineUpdate::Usage(msg) => msg,
                };
                self.log_command("statusline", message);
            }
            "permissions" => {
                let message = match PermissionMode::from_label(&args) {
                    Some(mode) => {
                        self.permission_mode.set(mode);
                        format!("mode set to {}", mode.label())
                    }
                    None if args.is_empty() => {
                        format!("mode is {}", self.permission_mode.get().label())
                    }
                    None => {
                        "usage: /permissions [ask|accept-edits|bypass|plan]".to_string()
                    }
                };
                self.log_command("permissions", message);
            }
            "sandbox" => {
                let sandbox = self.banner.sandbox.label().to_string();
                self.log_command("sandbox", format!("profile is {sandbox}"));
            }
            "status" => {
                let status = format!(
                    "{}/{} · sandbox: {} · ctx {}% ({}/{} tokens) · {}",
                    self.active_provider,
                    self.active_model,
                    self.banner.sandbox.label(),
                    self.memory_percent(),
                    self.current_tokens,
                    self.max_context_tokens,
                    self.permission_mode.get().label(),
                );
                self.log_command("status", status);
            }
            "clear" => {
                self.logs.clear();
                self.assistant_text.clear();
            }
            "help" => {
                for cmd in self.slash_registry.list() {
                    self.logs
                        .push(format!("/{} — {}", cmd.name, cmd.description));
                }
            }
            "quit" | "exit" => return SlashOutcome::Quit,
            "resume" => {
                if args.is_empty() {
                    let store =
                        cust_session::SessionStore::new(cust_session::SessionStore::default_dir());
                    match store.list_sessions() {
                        Ok(sessions) if sessions.is_empty() => {
                            self.log_command("resume", "no saved sessions found".to_string());
                        }
                        Ok(sessions) => {
                            self.log_command("resume", "usage: /resume <id>".to_string());
                            for s in sessions {
                                self.logs.push(format!("  - [{}] {}", s.id, s.title));
                            }
                        }
                        Err(e) => {
                            self.log_command("resume", format!("failed to list sessions: {e}"));
                        }
                    }
                    return SlashOutcome::Consumed;
                }
                let store =
                    cust_session::SessionStore::new(cust_session::SessionStore::default_dir());
                match store.load_session(&args) {
                    Ok((meta, messages)) => {
                        self.log_command(
                            "resume",
                            format!("resumed '{}' ({} messages)", meta.title, messages.len()),
                        );
                        for msg in &messages {
                            self.logs.push(format!("{}: {}", msg.role, msg.content));
                        }
                    }
                    Err(e) => {
                        self.log_command("resume", format!("failed to resume '{args}': {e}"));
                    }
                }
                return SlashOutcome::Consumed;
            }
            // Commands with a canned prompt, then anything unrecognised.
            _ => {
                let prompt = self
                    .slash_registry
                    .expand(input)
                    .unwrap_or_else(|| input.to_string());
                return SlashOutcome::Prompt(prompt);
            }
        }
        SlashOutcome::Consumed
    }

    fn log_command(&mut self, name: &str, message: impl std::fmt::Display) {
        self.logs.push(format!("[/{name}] {message}"));
    }

    /// Advance to the next permission mode (Shift+Tab) and log the change.
    pub fn cycle_permission_mode(&mut self) -> PermissionMode {
        let mode = self.permission_mode.cycle();
        self.logs.push(format!("[Permission] {}", mode.label()));
        mode
    }

    /// Point the banner at the session's provider/model and sandbox setting.
    pub fn set_session_info(
        &mut self,
        provider: impl Into<String>,
        model: impl Into<String>,
        sandbox: crate::banner::SandboxStatus,
    ) {
        self.active_provider = provider.into();
        self.active_model = model.into();
        self.banner.provider = self.active_provider.clone();
        self.banner.model = self.active_model.clone();
        self.banner.sandbox = sandbox;
    }

    pub fn memory_percent(&self) -> u16 {
        if self.max_context_tokens == 0 {
            0
        } else {
            ((self.current_tokens as f64 / self.max_context_tokens as f64) * 100.0).min(100.0) as u16
        }
    }

    pub fn on_input_change(&mut self, text: String) {
        self.input_buffer = text;
        if self.input_buffer.starts_with('/') {
            self.slash_suggestions = self
                .slash_registry
                .match_prefix(&self.input_buffer)
                .into_iter()
                .map(|cmd| format!("/{} — {}", cmd.name, cmd.description))
                .collect();
        } else {
            self.slash_suggestions.clear();
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        match event.kind {
            EventKind::TurnStarted { turn } => {
                self.status = format!("Running (Turn #{turn})");
                self.show_banner = false;
            }
            EventKind::AssistantDelta { text } => {
                self.assistant_text.push_str(&text);
                self.current_tokens += text.len() / 4;
            }
            EventKind::ReasoningDelta { text } => {
                self.assistant_text.push_str(&text);
                self.current_tokens += text.len() / 4;
            }
            EventKind::ToolCall { tool, args, .. } => {
                self.logs.push(format!("[Tool Call] {tool}({args})"));
            }
            EventKind::ToolResult { ok, summary, .. } => {
                let status_str = if ok { "OK" } else { "FAIL" };
                self.logs
                    .push(format!("[Tool Result: {status_str}] {summary}"));
            }
            EventKind::TurnEnded { turn: _, reason } => {
                self.status = format!("Ended ({reason})");
            }
            EventKind::Error { message, .. } => {
                self.status = format!("Error: {message}");
                self.logs.push(format!("[Error] {message}"));
            }
            _ => {}
        }
    }
}
