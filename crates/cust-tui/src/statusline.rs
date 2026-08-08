//! The status line: a one-row strip of segments (model, workspace, branch,
//! context use, permission mode) drawn at the bottom of the TUI.
//!
//! `/statusline` configures it at runtime — see [`StatusLineConfig::apply`].

use crate::app::TuiState;
use crate::permission::PermissionMode;
use crate::ink::utils::truncate_to_width;

/// Which segments the status line shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusLineConfig {
    pub enabled: bool,
    pub model: bool,
    pub workspace: bool,
    pub branch: bool,
    pub context: bool,
    pub permission: bool,
}

impl Default for StatusLineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: true,
            workspace: true,
            branch: true,
            context: true,
            permission: true,
        }
    }
}

/// Outcome of a `/statusline` invocation, reported back to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusLineUpdate {
    /// The config changed; the message describes how.
    Changed(String),
    /// The input was not understood; the message is usage help.
    Usage(String),
}

impl StatusLineConfig {
    /// Apply a `/statusline` argument string.
    ///
    /// Accepted forms: `on`, `off`, `reset`, `<segment> on|off`, and a bare
    /// `<segment>` to toggle it. Segments: model, workspace, branch, context,
    /// permission.
    pub fn apply(&mut self, args: &str) -> StatusLineUpdate {
        let mut words = args.split_whitespace();
        let Some(head) = words.next() else {
            self.enabled = !self.enabled;
            return StatusLineUpdate::Changed(format!(
                "status line {}",
                on_off(self.enabled)
            ));
        };
        let value = words.next();

        match head.to_ascii_lowercase().as_str() {
            "on" | "show" => {
                self.enabled = true;
                StatusLineUpdate::Changed("status line on".to_string())
            }
            "off" | "hide" => {
                self.enabled = false;
                StatusLineUpdate::Changed("status line off".to_string())
            }
            "reset" => {
                *self = Self::default();
                StatusLineUpdate::Changed("status line reset to defaults".to_string())
            }
            segment => {
                let Some(field) = self.segment_mut(segment) else {
                    return StatusLineUpdate::Usage(
                        "usage: /statusline [on|off|reset] | <model|workspace|branch|context|permission> [on|off]"
                            .to_string(),
                    );
                };
                *field = match value.map(|v| v.to_ascii_lowercase()) {
                    Some(v) if v == "on" => true,
                    Some(v) if v == "off" => false,
                    // A bare segment name toggles it.
                    None => !*field,
                    Some(_) => {
                        return StatusLineUpdate::Usage(format!(
                            "usage: /statusline {segment} [on|off]"
                        ));
                    }
                };
                let state = on_off(*field);
                StatusLineUpdate::Changed(format!("status line {segment} {state}"))
            }
        }
    }

    fn segment_mut(&mut self, name: &str) -> Option<&mut bool> {
        match name {
            "model" => Some(&mut self.model),
            "workspace" | "dir" | "cwd" => Some(&mut self.workspace),
            "branch" | "git" => Some(&mut self.branch),
            "context" | "ctx" => Some(&mut self.context),
            "permission" | "permissions" | "perm" => Some(&mut self.permission),
            _ => None,
        }
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

/// The status line as plain text, segments joined by `·`. Used for tests and
/// for any non-TUI surface that wants the same summary.
pub fn render_text(state: &TuiState, config: &StatusLineConfig) -> String {
    if !config.enabled {
        return String::new();
    }
    let mut parts: Vec<String> = Vec::new();
    if config.model {
        parts.push(format!("✦ {}", state.active_model));
    }
    if config.workspace {
        parts.push(format!("▪ {}", state.workspace));
    }
    if config.branch && let Some(branch) = &state.git_branch {
        parts.push(format!("⑂ {branch}"));
    }
    if config.context {
        parts.push(format!("ctx {}%", state.memory_percent()));
    }
    if config.permission {
        parts.push(state.permission_mode.get().footer());
    }
    parts.join("  ·  ")
}

const ANSI_RESET: &str = "\u{1b}[0m";
const ANSI_BOLD: &str = "\u{1b}[1m";
const ANSI_RED: &str = "\u{1b}[31m";
const ANSI_GREEN: &str = "\u{1b}[32m";
const ANSI_YELLOW: &str = "\u{1b}[33m";
const ANSI_BLUE: &str = "\u{1b}[34m";
const ANSI_MAGENTA: &str = "\u{1b}[35m";
const ANSI_CYAN: &str = "\u{1b}[36m";
const ANSI_DARK_GRAY: &str = "\u{1b}[90m";

/// Render the status line as one ANSI-styled row, padded to `width`.
///
/// An empty vec means the line is disabled — callers should reserve zero rows
/// for it, matching the old ratatui layout where a hidden statusline took no
/// space.
pub fn render(state: &TuiState, config: &StatusLineConfig, width: usize) -> Vec<String> {
    if !config.enabled {
        return Vec::new();
    }

    let mut line = String::new();
    let push = |line: &mut String, text: String, color: &str| {
        if !line.is_empty() {
            line.push_str(ANSI_DARK_GRAY);
            line.push_str("  \u{b7}  ");
            line.push_str(ANSI_RESET);
        }
        line.push_str(color);
        line.push_str(ANSI_BOLD);
        line.push_str(&text);
        line.push_str(ANSI_RESET);
    };

    if config.model {
        push(&mut line, format!("\u{2726} {}", state.active_model), ANSI_MAGENTA);
    }
    if config.workspace {
        push(&mut line, format!("\u{25aa} {}", state.workspace), ANSI_CYAN);
    }
    if config.branch {
        if let Some(branch) = &state.git_branch {
            push(&mut line, format!("\u{2442} {branch}"), ANSI_BLUE);
        }
    }
    if config.context {
        let percent = state.memory_percent();
        let color = if percent > 80 { ANSI_RED } else { ANSI_GREEN };
        push(&mut line, format!("ctx {percent}%"), color);
    }
    if config.permission {
        let mode = state.permission_mode.get();
        let color = match mode {
            PermissionMode::BypassPermissions => ANSI_RED,
            PermissionMode::AcceptEdits => ANSI_YELLOW,
            PermissionMode::Ask | PermissionMode::Plan => ANSI_DARK_GRAY,
        };
        push(&mut line, mode.footer(), color);
    }

    // A narrow terminal gets a clipped-with-ellipsis line rather than one
    // that overflows the requested width — the caller's differ assumes every
    // row respects it.
    vec![truncate_to_width(&line, width, Some("\u{2026}")).text]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_command_toggles_the_whole_line() {
        let mut config = StatusLineConfig::default();
        assert_eq!(
            config.apply(""),
            StatusLineUpdate::Changed("status line off".to_string())
        );
        assert!(!config.enabled);
        assert_eq!(
            config.apply(""),
            StatusLineUpdate::Changed("status line on".to_string())
        );
    }

    #[test]
    fn segments_accept_on_off_and_bare_toggles() {
        let mut config = StatusLineConfig::default();
        config.apply("branch off");
        assert!(!config.branch);
        config.apply("branch");
        assert!(config.branch);
        // Aliases resolve to the same segment.
        config.apply("ctx off");
        assert!(!config.context);
        config.apply("reset");
        assert_eq!(config, StatusLineConfig::default());
    }

    #[test]
    fn unknown_segments_return_usage_and_change_nothing() {
        let mut config = StatusLineConfig::default();
        let before = config;
        assert!(matches!(
            config.apply("bogus on"),
            StatusLineUpdate::Usage(_)
        ));
        assert!(matches!(
            config.apply("branch maybe"),
            StatusLineUpdate::Usage(_)
        ));
        assert_eq!(config, before);
    }

    #[test]
    fn text_rendering_honours_the_enabled_segments() {
        let mut state = TuiState::default();
        state.workspace = "cust-code".to_string();
        state.git_branch = Some("main".to_string());
        state.current_tokens = 12_800;
        state.max_context_tokens = 128_000;

        let mut config = StatusLineConfig::default();
        let full = render_text(&state, &config);
        assert!(full.contains("✦ gpt-4o"), "{full}");
        assert!(full.contains("▪ cust-code"), "{full}");
        assert!(full.contains("⑂ main"), "{full}");
        assert!(full.contains("ctx 10%"), "{full}");
        assert!(full.contains("shift+tab to cycle"), "{full}");

        config.apply("branch off");
        assert!(!render_text(&state, &config).contains("⑂ main"));

        config.apply("off");
        assert_eq!(render_text(&state, &config), "");
    }
}
