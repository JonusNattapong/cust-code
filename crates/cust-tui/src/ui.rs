//! Composes the whole TUI frame: banner, header, context gauge, body,
//! input box, and status line — each an `ink` panel stacked top to bottom.
//!
//! Replaces the six-pane ratatui `Layout` (37e): instead of fixed-height
//! chunks in an alternate screen, this returns the frame as plain lines that
//! the caller diffs and writes inline, leaving finished output in the
//! terminal's own scrollback.

use crate::app::{TuiState, ViewMode};
use crate::ink::components::{BorderStyle, BoxView, Text};
use crate::ink::utils::pad_to_width;
use crate::ink::Component;
use crate::theme;

fn bordered_panel(title: &str, body: &str, width: usize) -> Vec<String> {
    let mut panel = BoxView::new()
        .with_padding(1, 0)
        .with_border(BorderStyle::Single)
        .with_title(title)
        .with_border_color(theme::primary());
    panel.add_child(Box::new(Text::new(body).with_padding(0, 0)));
    panel.render(width)
}

/// A single-row percentage gauge: `[████░░░░] 42% (12,800/128,000 tokens)`.
fn context_gauge(percent: u16, current: usize, max: usize, width: usize) -> String {
    let bar_width = 20usize;
    let filled = (bar_width * percent.min(100) as usize) / 100;
    let bar = "\u{2588}".repeat(filled) + &"\u{2591}".repeat(bar_width - filled);
    let color = if percent > 80 { theme::DANGER.to_string() } else { theme::primary() };
    let bold = theme::BOLD;
    let reset = theme::RESET;
    let text = format!("{color}{bold}[{bar}] {percent}% ({current}/{max} tokens){reset}");
    pad_to_width(&text, width)
}

/// Render the full frame at `width`, returning one string per row.
///
/// Callers own the height budget: pass this straight to a differential
/// renderer, which only repaints the rows that actually changed between
/// calls.
pub fn render(state: &TuiState, width: usize) -> Vec<String> {
    let mut out = Vec::new();

    if state.show_banner {
        out.extend(crate::banner::render(&state.banner, width));
    }

    // 1. Header dashboard
    let mode_str = match state.view_mode {
        ViewMode::Agent => "[Agent Mode]",
        ViewMode::TerminalShell => "[Shell Mode]",
    };
    let header_body = format!(
        "{}Status: {} | Model: {} ({}) | Mode: {}{}",
        theme::primary(), state.status, state.active_model, state.active_provider, mode_str, theme::RESET
    );
    out.extend(bordered_panel("Header Dashboard", &header_body, width));

    // 2. Live context memory gauge
    let percent = state.memory_percent();
    out.extend(bordered_panel(
        "Context Window Memory Budget",
        &context_gauge(percent, state.current_tokens, state.max_context_tokens, width.saturating_sub(4)),
        width,
    ));

    // 3. Body: assistant output + logs
    let body_text = if state.assistant_text.is_empty() {
        state.logs.join("\n")
    } else {
        format!("{}\n\n--- Logs ---\n{}", state.assistant_text, state.logs.join("\n"))
    };
    let body_title = match state.view_mode {
        ViewMode::Agent => "Assistant & Tool Execution Stream",
        ViewMode::TerminalShell => "Terminal Shell Output Stream",
    };
    out.extend(bordered_panel(body_title, &body_text, width));

    // 4. Input box (and slash-command suggestions, if any)
    let input_title = if state.slash_suggestions.is_empty() {
        "Input Prompt (Type / for Slash Commands)"
    } else {
        "Slash Commands Autocomplete Menu"
    };
    let input_display = if state.slash_suggestions.is_empty() {
        format!("{}{}{}", theme::primary(), state.input_buffer, theme::RESET)
    } else {
        format!(
            "{}{}{}\nSuggestions:\n{}",
            theme::primary(),
            state.input_buffer,
            theme::RESET,
            state.slash_suggestions.join(" | ")
        )
    };
    out.extend(bordered_panel(input_title, &input_display, width));

    // 5. Status line — model, workspace, branch, context, permission mode.
    out.extend(crate::statusline::render(state, &state.statusline, width));

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::utils::{strip_ansi, visible_width};

    #[test]
    fn renders_every_panel_at_a_reasonable_width() {
        let state = TuiState::default();
        let lines = render(&state, 80);
        let text = strip_ansi(&lines.join("\n"));
        assert!(text.contains("Header Dashboard"));
        assert!(text.contains("Context Window Memory Budget"));
        assert!(text.contains("Input Prompt"));
    }

    #[test]
    fn every_row_fits_inside_the_requested_width() {
        let state = TuiState::default();
        for width in [30usize, 50, 80, 120] {
            for line in render(&state, width) {
                assert!(
                    visible_width(&line) <= width,
                    "row wider than {width}: {line:?}"
                );
            }
        }
    }

    #[test]
    fn banner_shows_on_a_fresh_session_and_disappears_after_dismissal() {
        let mut state = TuiState::default();
        let with_banner = strip_ansi(&render(&state, 80).join("\n"));
        assert!(with_banner.contains("Welcome"));

        state.show_banner = false;
        let without = strip_ansi(&render(&state, 80).join("\n"));
        assert!(!without.contains("Welcome"));
        assert!(without.contains("Header Dashboard"));
    }

    #[test]
    fn permission_footer_reflects_the_current_mode() {
        let mut state = TuiState::default();
        let ask = strip_ansi(&render(&state, 80).join("\n"));
        assert!(
            ask.contains("shift+tab to cycle"),
            "default mode footer missing:\n{ask}"
        );

        state.cycle_permission_mode();
        state.cycle_permission_mode();
        let bypass = strip_ansi(&render(&state, 80).join("\n"));
        assert!(bypass.contains("bypass permissions"), "{bypass}");
    }
}
