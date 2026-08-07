//! The status line: a one-row strip of segments (model, workspace, branch,
//! context use, permission mode) drawn at the bottom of the TUI.
//!
//! `/statusline` configures it at runtime — see [`StatusLineConfig::apply`].

use crate::app::TuiState;
use crate::permission::PermissionMode;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

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

/// Draw the status line into a single-row `area`.
pub fn render(frame: &mut Frame<'_>, area: Rect, state: &TuiState, config: &StatusLineConfig) {
    if !config.enabled {
        return;
    }

    let mut spans: Vec<Span<'_>> = Vec::new();
    let mut push = |text: String, color: Color| {
        if !spans.is_empty() {
            spans.push(Span::styled("  ·  ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(
            text,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    };

    if config.model {
        push(format!("✦ {}", state.active_model), Color::Magenta);
    }
    if config.workspace {
        push(format!("▪ {}", state.workspace), Color::Cyan);
    }
    if config.branch && let Some(branch) = &state.git_branch {
        push(format!("⑂ {branch}"), Color::Blue);
    }
    if config.context {
        let percent = state.memory_percent();
        let color = if percent > 80 { Color::Red } else { Color::Green };
        push(format!("ctx {percent}%"), color);
    }
    if config.permission {
        let mode = state.permission_mode.get();
        let color = match mode {
            PermissionMode::BypassPermissions => Color::Red,
            PermissionMode::AcceptEdits => Color::Yellow,
            PermissionMode::Ask | PermissionMode::Plan => Color::DarkGray,
        };
        push(mode.footer(), color);
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
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
