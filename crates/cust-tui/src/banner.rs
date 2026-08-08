//! ASCII banner & welcome screen.
//!
//! Wide terminals get a two-column layout — welcome message + logo on the
//! left, a "Tips for getting started" list on the right — echoing the
//! Claude Code / Clew Code welcome screens. Narrow terminals fall back to a
//! single stacked column: full block art, then a compact wordmark, then a
//! single line, same degradation as before.

use crate::ink::components::{BorderStyle, BoxView, Columns, Text};
use crate::ink::Component;

/// Below this width there isn't room for two columns side by side; the
/// layout collapses to the original single-column banner.
const TWO_COLUMN_MIN_WIDTH: usize = 60;

/// Sandbox state shown in the welcome header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SandboxStatus {
    /// No isolation — tools run with full host access.
    #[default]
    Off,
    /// Filesystem writes restricted to the workspace.
    Workspace,
    /// Read-only filesystem, no network.
    ReadOnly,
    /// Fully isolated: no network, no writes outside a temp dir.
    Strict,
}

impl SandboxStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Workspace => "workspace-write",
            Self::ReadOnly => "read-only",
            Self::Strict => "strict",
        }
    }

    /// Parse a profile name (`off`, `workspace-write`, `read-only`, `strict`).
    /// Unknown names fall back to [`SandboxStatus::Off`].
    pub fn from_label(label: &str) -> Self {
        match label.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "workspace" | "workspace-write" => Self::Workspace,
            "read-only" | "readonly" => Self::ReadOnly,
            "strict" | "sealed" => Self::Strict,
            _ => Self::Off,
        }
    }

    /// ANSI foreground color code for this status, as used in [`render`].
    fn color_code(self) -> &'static str {
        match self {
            Self::Off => ANSI_RED,
            Self::Workspace => ANSI_YELLOW,
            Self::ReadOnly | Self::Strict => ANSI_GREEN,
        }
    }
}

const ANSI_RESET: &str = "\u{1b}[0m";
const ANSI_BOLD: &str = "\u{1b}[1m";
const ANSI_RED: &str = "\u{1b}[31m";
const ANSI_GREEN: &str = "\u{1b}[32m";
const ANSI_YELLOW: &str = "\u{1b}[33m";
const ANSI_WHITE: &str = "\u{1b}[37m";
const ANSI_MAGENTA: &str = "\u{1b}[35m";
const ANSI_CYAN: &str = "\u{1b}[36m";
const ANSI_DIM: &str = "\u{1b}[2m";

/// A single keyboard shortcut shown in the welcome guide.
#[derive(Debug, Clone, Copy)]
pub struct Shortcut {
    pub keys: &'static str,
    pub description: &'static str,
}

/// The shortcuts shown when no custom set is supplied.
pub const DEFAULT_SHORTCUTS: &[Shortcut] = &[
    Shortcut { keys: "Ctrl+X", description: "terminal shell" },
    Shortcut { keys: "/", description: "slash commands" },
    Shortcut { keys: "Shift+Tab", description: "cycle permissions" },
    Shortcut { keys: "Ctrl+C", description: "cancel turn" },
    Shortcut { keys: "Ctrl+D", description: "quit" },
];

/// The right-column tips shown when no custom list is supplied.
pub const DEFAULT_TIPS: &[&str] = &[
    "Run /init to create a project guide",
    "Type / to see all slash commands",
    "Ctrl+X toggles the terminal shell",
    "Shift+Tab cycles permission modes",
];

/// Everything the banner needs to know about the current session.
#[derive(Debug, Clone)]
pub struct BannerInfo {
    pub version: String,
    pub model: String,
    pub provider: String,
    pub sandbox: SandboxStatus,
    pub shortcuts: Vec<Shortcut>,
    /// Shown as "Welcome back {name}!" in the two-column layout. `None`
    /// falls back to a plain "Welcome".
    pub user_name: Option<String>,
    /// Current working directory, shown under the welcome message.
    pub workspace_path: Option<String>,
    /// Right-column bullet list under "Tips for getting started".
    pub tips: Vec<String>,
}

impl Default for BannerInfo {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            model: "unknown".to_string(),
            provider: "unknown".to_string(),
            sandbox: SandboxStatus::default(),
            shortcuts: DEFAULT_SHORTCUTS.to_vec(),
            user_name: None,
            workspace_path: None,
            tips: DEFAULT_TIPS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Which logo variant fits a given width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoSize {
    /// Five-row block letters (needs >= 44 columns).
    Full,
    /// Three-row compact wordmark (needs >= 28 columns).
    Compact,
    /// Single-line wordmark.
    Minimal,
}

impl LogoSize {
    /// Pick the largest variant that fits `width` columns.
    pub fn for_width(width: u16) -> Self {
        match width {
            w if w >= 44 => Self::Full,
            w if w >= 28 => Self::Compact,
            _ => Self::Minimal,
        }
    }
}

const LOGO_FULL: &[&str] = &[
    r"  ___ _   _ ___ _____ ",
    r" / __| | | / __|_   _|",
    r"| (__| |_| \__ \ | |  ",
    r" \___|\___/|___/ |_|  ",
    r"      c o d e         ",
];

const LOGO_COMPACT: &[&str] = &[
    r"┌─┐┬ ┬┌─┐┌┬┐",
    r"│  │ │└─┐ │ ",
    r"└─┘└─┘└─┘ ┴  code",
];

const LOGO_MINIMAL: &[&str] = &["cust ▸ code"];

/// The logo rows for a size variant.
pub fn logo_lines(size: LogoSize) -> &'static [&'static str] {
    match size {
        LogoSize::Full => LOGO_FULL,
        LogoSize::Compact => LOGO_COMPACT,
        LogoSize::Minimal => LOGO_MINIMAL,
    }
}

/// `v0.1.0 · openai/gpt-4o · sandbox: off`
fn status_line(info: &BannerInfo) -> String {
    format!(
        "v{} · {}/{} · sandbox: {}",
        info.version,
        info.provider,
        info.model,
        info.sandbox.label()
    )
}

/// Shortcut hints packed into lines no wider than `width`.
fn shortcut_lines(shortcuts: &[Shortcut], width: u16) -> Vec<String> {
    const SEP: &str = "   ";
    let width = width.max(1) as usize;
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for hint in shortcuts.iter().map(|s| format!("{} {}", s.keys, s.description)) {
        if current.is_empty() {
            current = hint;
        } else if current.len() + SEP.len() + hint.len() <= width {
            current.push_str(SEP);
            current.push_str(&hint);
        } else {
            lines.push(std::mem::take(&mut current));
            current = hint;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Terminal width, falling back to 80 columns when stdout is not a tty.
pub fn terminal_width() -> u16 {
    crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80)
}

/// Render the whole banner as plain text — used by the non-TUI CLI entrypoint.
pub fn render_text(info: &BannerInfo, width: u16) -> String {
    let mut out = String::new();
    if let Some(name) = &info.user_name {
        out.push_str(&format!("Welcome back {name}!\n\n"));
    }
    for line in logo_lines(LogoSize::for_width(width)) {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&status_line(info));
    out.push('\n');
    for line in shortcut_lines(&info.shortcuts, width) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Column width [`render`] gives the left (welcome/logo) column at a given
/// panel content width.
fn left_column_width(content_width: u16) -> u16 {
    content_width * 3 / 5
}

/// Number of rows [`render`] needs at a given width, including the border.
///
/// Delegates to [`render`] itself rather than re-deriving the row count by
/// hand: at narrow widths the welcome/status/tips text word-wraps, and
/// predicting exactly how many rows that produces means re-implementing
/// `wrap_text_with_ansi`'s line-breaking. Rendering a few dozen short
/// strings is cheap; a second, hand-maintained implementation that quietly
/// drifts from the real one is not a good trade for the difference.
pub fn height_for(info: &BannerInfo, width: u16) -> u16 {
    render(info, width as usize).len() as u16
}

fn welcome_body(info: &BannerInfo) -> String {
    match &info.user_name {
        Some(name) => format!("{ANSI_BOLD}Welcome back {name}!{ANSI_RESET}"),
        None => format!("{ANSI_BOLD}Welcome{ANSI_RESET}"),
    }
}

/// Build the left column: welcome line, logo, and the version/model/sandbox
/// status line.
fn left_column(info: &BannerInfo, width: usize) -> String {
    let mut body = String::new();
    body.push_str(&welcome_body(info));
    body.push_str("\n\n");

    for row in logo_lines(LogoSize::for_width(width as u16)) {
        body.push_str(ANSI_CYAN);
        body.push_str(ANSI_BOLD);
        body.push_str(row);
        body.push_str(ANSI_RESET);
        body.push('\n');
    }
    body.push('\n');

    body.push_str(ANSI_WHITE);
    body.push_str(&format!("v{}", info.version));
    body.push_str(ANSI_RESET);
    body.push_str(" \u{b7} ");
    body.push_str(ANSI_MAGENTA);
    body.push_str(&format!("{}/{}", info.provider, info.model));
    body.push_str(ANSI_RESET);
    body.push_str(" \u{b7} sandbox: ");
    body.push_str(info.sandbox.color_code());
    body.push_str(ANSI_BOLD);
    body.push_str(info.sandbox.label());
    body.push_str(ANSI_RESET);

    if let Some(path) = &info.workspace_path {
        body.push('\n');
        body.push_str(ANSI_DIM);
        body.push_str(path);
        body.push_str(ANSI_RESET);
    }

    body
}

/// Build the right column: a "Tips for getting started" heading and bullets.
fn tips_column(info: &BannerInfo) -> String {
    let mut body = format!("{ANSI_BOLD}Tips for getting started{ANSI_RESET}\n\n");
    for tip in &info.tips {
        body.push_str(ANSI_DIM);
        body.push_str("\u{2022} ");
        body.push_str(ANSI_RESET);
        body.push_str(tip);
        body.push('\n');
    }
    body.trim_end().to_string()
}

/// Render the banner as a bordered `ink` panel.
///
/// At `width >= 60` this is a two-column layout (welcome + logo on the left,
/// tips on the right), matching the Claude Code / Clew Code welcome screens.
/// Narrower terminals fall back to the original single stacked column, since
/// two columns of anything useful don't fit below that width.
///
/// Width is passed explicitly rather than pulled from the terminal because
/// `ink` components render on demand — the caller supplies the same width it
/// will hand the rest of the frame.
pub fn render(info: &BannerInfo, width: usize) -> Vec<String> {
    let mut panel = BoxView::new()
        .with_padding(1, 0)
        .with_border(BorderStyle::Rounded)
        .with_title("Welcome");

    // The border and the panel's own padding (1 column each side) both eat
    // into what children get to wrap at — matches BoxView's internal
    // `width - (padding_x + border_inset) * 2`. Computed once here so the
    // pre-rendered `Columns` output and the width BoxView hands its wrapping
    // Text child never drift apart (they did, briefly: a wider precomputed
    // row than the child's actual wrap width caused it to re-wrap and add
    // spurious rows).
    let content_width = width.saturating_sub(4);

    if content_width >= TWO_COLUMN_MIN_WIDTH {
        let left_width = left_column_width(content_width as u16) as usize;
        let left = Text::new(left_column(info, left_width)).with_padding(0, 0);
        let right = Text::new(tips_column(info)).with_padding(0, 0);
        let mut columns = Columns::new(vec![Box::new(left), Box::new(right)]).with_weights(vec![3, 2]);
        let column_rows = columns.render(content_width);
        panel.add_child(Box::new(Text::new(column_rows.join("\n")).with_padding(0, 0)));
    } else {
        panel.add_child(Box::new(Text::new(left_column(info, content_width)).with_padding(0, 0)));
    }

    let mut hints = String::new();
    for hint in shortcut_lines(&info.shortcuts, content_width as u16) {
        hints.push_str(ANSI_DIM);
        hints.push_str(&hint);
        hints.push_str(ANSI_RESET);
        hints.push('\n');
    }
    panel.add_child(Box::new(Text::new(hints.trim_end_matches('\n')).with_padding(0, 0)));

    panel.render(width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::utils::{strip_ansi, visible_width};

    fn info() -> BannerInfo {
        BannerInfo {
            version: "0.0.0".to_string(),
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            sandbox: SandboxStatus::Workspace,
            shortcuts: DEFAULT_SHORTCUTS.to_vec(),
            user_name: None,
            workspace_path: None,
            tips: DEFAULT_TIPS.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn logo_size_follows_width() {
        assert_eq!(LogoSize::for_width(120), LogoSize::Full);
        assert_eq!(LogoSize::for_width(44), LogoSize::Full);
        assert_eq!(LogoSize::for_width(43), LogoSize::Compact);
        assert_eq!(LogoSize::for_width(28), LogoSize::Compact);
        assert_eq!(LogoSize::for_width(10), LogoSize::Minimal);
    }

    #[test]
    fn logo_rows_fit_their_minimum_width() {
        // Each variant's rows must fit within the width that selects it.
        for (size, min) in [(LogoSize::Full, 44), (LogoSize::Compact, 28), (LogoSize::Minimal, 27)] {
            for row in logo_lines(size) {
                assert!(
                    row.chars().count() <= min,
                    "{size:?} row {row:?} exceeds {min} columns"
                );
            }
        }
    }

    #[test]
    fn text_banner_includes_version_model_and_sandbox() {
        let text = render_text(&info(), 80);
        assert!(text.contains("v0.0.0"));
        assert!(text.contains("openai/gpt-4o"));
        assert!(text.contains("sandbox: workspace-write"));
        assert!(text.contains("Ctrl+X terminal shell"));
        assert!(text.contains("/ slash commands"));
    }

    #[test]
    fn text_banner_greets_a_named_user() {
        let mut i = info();
        i.user_name = Some("Nattapong".to_string());
        let text = render_text(&i, 80);
        assert!(text.contains("Welcome back Nattapong!"));
    }

    #[test]
    fn shortcuts_wrap_to_the_available_width() {
        let wide = shortcut_lines(DEFAULT_SHORTCUTS, 200);
        assert_eq!(wide.len(), 1);

        let narrow = shortcut_lines(DEFAULT_SHORTCUTS, 24);
        assert!(narrow.len() > 1);
        for line in &narrow {
            assert!(line.chars().count() <= 24 || !line.contains("   "));
        }
    }

    #[test]
    fn narrow_panel_falls_back_to_single_column() {
        let rows: Vec<String> = render(&info(), 40).iter().map(|l| strip_ansi(l)).collect();
        let text = rows.join("\n");
        assert!(!text.contains("Tips for getting started"));
        assert!(text.contains("code"));
    }

    #[test]
    fn wide_panel_shows_two_columns() {
        let rows: Vec<String> = render(&info(), 100).iter().map(|l| strip_ansi(l)).collect();
        let text = rows.join("\n");
        assert!(text.contains("Tips for getting started"));
        assert!(text.contains(r"\___|\___/|___/"), "left column logo missing:\n{text}");

        // Proof the columns sit side by side rather than stacked: the "Tips"
        // heading (top of the right column) shares vertical space with the
        // logo (middle of the left column) instead of appearing only after
        // the whole left column has scrolled past.
        let tips_row = rows.iter().position(|l| l.contains("Tips for getting started")).expect("tips heading row");
        let last_logo_row = rows
            .iter()
            .position(|l| l.contains(LOGO_FULL[4].trim_end()))
            .expect("last logo row");
        assert!(
            tips_row < last_logo_row,
            "tips heading (row {tips_row}) should appear beside the logo (through row {last_logo_row}), not after it"
        );

        for row in &rows {
            assert!(visible_width(row) <= 100, "row wider than 100: {row:?}");
        }
    }

    #[test]
    fn wide_panel_greets_a_named_user_in_the_left_column() {
        let mut i = info();
        i.user_name = Some("Nattapong".to_string());
        let text = render(&i, 100).iter().map(|l| strip_ansi(l)).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Welcome back Nattapong!"));
    }

    #[test]
    fn wide_panel_shows_custom_tips() {
        let mut i = info();
        i.tips = vec!["Run /doctor to check your setup".to_string()];
        let text = render(&i, 100).iter().map(|l| strip_ansi(l)).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Run /doctor to check your setup"));
    }

    #[test]
    fn height_matches_rendered_row_count() {
        let info = info();
        for width in [20u16, 40, 80, 120] {
            let rows = render(&info, width as usize).len() as u16;
            assert_eq!(height_for(&info, width), rows, "mismatch at width {width}");
        }
    }
}
