//! Buffer-level tests for the welcome banner panel across terminal widths.

use cust_tui::banner::{self, BannerInfo, SandboxStatus};
use cust_tui::{TuiState, ui};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

fn info() -> BannerInfo {
    BannerInfo {
        version: "1.2.3".to_string(),
        model: "gpt-4o".to_string(),
        provider: "openai".to_string(),
        sandbox: SandboxStatus::ReadOnly,
        shortcuts: banner::DEFAULT_SHORTCUTS.to_vec(),
    }
}

/// The drawn buffer, one string per row, trailing blanks trimmed.
fn draw<F: FnOnce(&mut ratatui::Frame<'_>)>(width: u16, height: u16, f: F) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| f(frame)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn render_banner_rows(width: u16) -> Vec<String> {
    let info = info();
    let height = banner::height_for(&info, width);
    draw(width, height, |frame| {
        banner::render(frame, Rect::new(0, 0, width, height), &info)
    })
}

#[test]
fn wide_terminal_draws_the_full_logo_and_status() {
    let rows = render_banner_rows(100);
    let text = rows.join("\n");

    assert!(text.contains("Welcome"), "missing panel title:\n{text}");
    // A distinctive slice of the five-row block art.
    assert!(text.contains(r"\___|\___/|___/"), "missing full logo:\n{text}");
    assert!(text.contains("v1.2.3"), "missing version:\n{text}");
    assert!(text.contains("openai/gpt-4o"), "missing model:\n{text}");
    assert!(text.contains("sandbox: read-only"), "missing sandbox:\n{text}");
    assert!(text.contains("Ctrl+X terminal shell"), "missing hints:\n{text}");
}

#[test]
fn narrow_terminal_degrades_to_a_smaller_logo() {
    let wide = render_banner_rows(100).join("\n");
    let narrow = render_banner_rows(30).join("\n");

    assert!(!narrow.contains(r"\___|\___/|___/"), "full logo at 30 cols:\n{narrow}");
    assert!(narrow.contains("code"), "missing wordmark:\n{narrow}");
    // The status line still gets through, just on more rows than the wide case.
    assert!(narrow.contains("v1.2.3"), "missing version:\n{narrow}");
    assert!(wide.lines().count() < narrow.lines().count());
}

#[test]
fn every_row_fits_inside_the_panel_border() {
    for width in [30u16, 50, 100] {
        for row in render_banner_rows(width) {
            assert!(
                row.chars().count() <= width as usize,
                "row wider than {width}: {row:?}"
            );
        }
    }
}

#[test]
fn permission_footer_shows_the_current_mode() {
    let mut state = TuiState::default();
    let ask = draw(80, 30, |frame| ui::render(frame, &state)).join("\n");
    assert!(
        ask.contains("⏵⏵ ask every time on (shift+tab to cycle)"),
        "default mode footer missing:\n{ask}"
    );

    // Shift+Tab twice: ask -> accept edits -> bypass permissions.
    state.cycle_permission_mode();
    state.cycle_permission_mode();
    let bypass = draw(80, 30, |frame| ui::render(frame, &state)).join("\n");
    assert!(
        bypass.contains("⏵⏵ bypass permissions on (shift+tab to cycle)"),
        "cycled mode footer missing:\n{bypass}"
    );
}

#[test]
fn banner_occupies_the_top_of_the_ui_then_disappears_after_a_turn() {
    let mut state = TuiState::default();
    let with_banner = draw(80, 30, |frame| ui::render(frame, &state));
    assert!(
        with_banner.join("\n").contains("Welcome"),
        "banner missing on a fresh session"
    );

    state.show_banner = false;
    let without = draw(80, 30, |frame| ui::render(frame, &state));
    assert!(
        !without.join("\n").contains("Welcome"),
        "banner still drawn after it was dismissed"
    );
    // The header takes over the top rows once the banner is gone.
    assert!(without.join("\n").contains("Header Dashboard"));
}
