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

/// The prompt: a horizontal rule, a `❯`-prefixed input row (plus suggestion
/// rows when slash completions are showing), then a closing rule.
///
/// Matches `clew-code`'s actual `PromptInput` framing — `borderLeft={false}
/// borderRight={false} borderBottom` — rather than a 4-sided box: side walls
/// around a single line of text read as a mistake (an empty box), not a
/// prompt, so `clew-code` never draws them there.
fn input_line(state: &TuiState, width: usize) -> Vec<String> {
    let rule = format!("{}{}{}", theme::primary(), "\u{2500}".repeat(width), theme::RESET);
    // Only the prompt glyph carries the theme color; typed text stays the
    // terminal's default foreground, matching clew-code's PromptChar (themed)
    // vs. the text input itself (untouched).
    let prompt = format!("{}{}\u{276f}{} {}", theme::primary(), theme::BOLD, theme::RESET, state.input_buffer);

    let mut out = vec![rule.clone(), pad_to_width(&prompt, width)];
    for suggestion in &state.slash_suggestions {
        out.push(pad_to_width(&format!("{}{}{}", theme::DIM, suggestion, theme::RESET), width));
    }
    out.push(rule);
    out
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

    // 4. Prompt input — horizontal rules, no side walls (see input_line).
    out.extend(input_line(state, width));

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
        // The prompt has no title/box — its presence is the ❯ row plus the
        // rules bracketing it, checked in input_line's own tests below.
        assert!(text.contains('\u{276f}'), "prompt glyph missing:\n{text}");
    }

    #[test]
    fn input_line_has_no_side_walls() {
        let mut state = TuiState::default();
        state.input_buffer = "hello".to_string();
        let lines = strip_ansi(&input_line(&state, 20).join("\n"));
        // A horizontal rule, not a box: no │ characters anywhere in the row.
        assert!(!lines.contains('\u{2502}'), "prompt should not draw side walls:\n{lines}");
        assert!(lines.contains("\u{2500}"), "missing the horizontal rule:\n{lines}");
        assert!(lines.contains("\u{276f} hello"), "missing the prompt + typed text:\n{lines}");
    }

    #[test]
    fn input_line_shows_slash_suggestions_between_the_rules() {
        let mut state = TuiState::default();
        state.input_buffer = "/h".to_string();
        state.slash_suggestions = vec!["/help — show help".to_string()];
        let rows = input_line(&state, 30);
        // rule, prompt, suggestion, rule
        assert_eq!(rows.len(), 4);
        assert!(strip_ansi(&rows[2]).contains("/help"));
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
