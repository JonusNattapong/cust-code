//! Row-level tests for the welcome banner panel and full frame across
//! terminal widths, rendered through `ink` rather than a ratatui backend.

use cust_tui::banner::{self, BannerInfo, SandboxStatus};
use cust_tui::ink::utils::{strip_ansi, visible_width};
use cust_tui::{ui, TuiState};

fn info() -> BannerInfo {
    BannerInfo {
        version: "1.2.3".to_string(),
        model: "gpt-4o".to_string(),
        provider: "openai".to_string(),
        sandbox: SandboxStatus::ReadOnly,
        shortcuts: banner::DEFAULT_SHORTCUTS.to_vec(),
        user_name: None,
        workspace_path: None,
        tips: banner::DEFAULT_TIPS.iter().map(|s| s.to_string()).collect(),
    }
}

fn render_banner_rows(width: u16) -> Vec<String> {
    banner::render(&info(), width as usize)
        .iter()
        .map(|l| strip_ansi(l))
        .collect()
}

#[test]
fn wide_terminal_draws_the_mascot_and_status() {
    let rows = render_banner_rows(100);
    let text = rows.join("\n");

    assert!(text.contains("Welcome"), "missing panel title:\n{text}");
    // The pixel-block mascot the wide two-column layout shows instead of
    // the text logo (narrow terminals keep the text logo — see
    // `narrow_terminal_degrades_to_a_smaller_logo`).
    assert!(text.contains("\u{2588}   \u{25bc}   \u{2588}"), "missing mascot:\n{text}");
    assert!(text.contains("v1.2.3"), "missing version:\n{text}");
    assert!(text.contains("openai/gpt-4o"), "missing model:\n{text}");
    assert!(text.contains("sandbox: read-only"), "missing sandbox:\n{text}");
    assert!(text.contains("Ctrl+X terminal shell"), "missing hints:\n{text}");
}

#[test]
fn narrow_terminal_degrades_to_a_smaller_logo() {
    let narrow = render_banner_rows(30).join("\n");

    assert!(!narrow.contains(r"\___|\___/|___/"), "full logo at 30 cols:\n{narrow}");
    assert!(narrow.contains("code"), "missing wordmark:\n{narrow}");
    // The status line still gets through, just on more rows than the wide case.
    assert!(narrow.contains("v1.2.3"), "missing version:\n{narrow}");
    // Below the two-column threshold, tips are dropped entirely rather than
    // squeezed in — there isn't room to lay them out beside the logo.
    assert!(
        !narrow.contains("Tips for getting started"),
        "narrow panel should not show the tips column:\n{narrow}"
    );
}

#[test]
fn every_row_fits_inside_the_panel_border() {
    for width in [30u16, 50, 100] {
        for row in banner::render(&info(), width as usize) {
            assert!(
                visible_width(&row) <= width as usize,
                "row wider than {width}: {row:?}"
            );
        }
    }
}

#[test]
fn permission_footer_shows_the_current_mode() {
    let mut state = TuiState::default();
    let ask = strip_ansi(&ui::render(&state, 80).join("\n"));
    assert!(
        ask.contains("⏵⏵ ask every time on (shift+tab to cycle)"),
        "default mode footer missing:\n{ask}"
    );

    // Shift+Tab twice: ask -> accept edits -> bypass permissions.
    state.cycle_permission_mode();
    state.cycle_permission_mode();
    let bypass = strip_ansi(&ui::render(&state, 80).join("\n"));
    assert!(
        bypass.contains("⏵⏵ bypass permissions on (shift+tab to cycle)"),
        "cycled mode footer missing:\n{bypass}"
    );
}

#[test]
fn banner_occupies_the_top_of_the_ui_then_disappears_after_a_turn() {
    let mut state = TuiState::default();
    let with_banner = strip_ansi(&ui::render(&state, 80).join("\n"));
    assert!(
        with_banner.contains("Welcome"),
        "banner missing on a fresh session"
    );

    state.show_banner = false;
    let without = strip_ansi(&ui::render(&state, 80).join("\n"));
    assert!(
        !without.contains("Welcome"),
        "banner still drawn after it was dismissed"
    );
    // The header takes over the top rows once the banner is gone.
    assert!(without.contains("Header Dashboard"));
}
