//! ASCII banner & welcome screen.
//!
//! Renders a flexible logo plus a status line (version, model, sandbox) and a
//! keyboard shortcut guide. The layout degrades gracefully as the terminal
//! narrows: full block art, then a compact wordmark, then a single line.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

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

    fn color(self) -> Color {
        match self {
            Self::Off => Color::Red,
            Self::Workspace => Color::Yellow,
            Self::ReadOnly | Self::Strict => Color::Green,
        }
    }
}

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

/// Everything the banner needs to know about the current session.
#[derive(Debug, Clone)]
pub struct BannerInfo {
    pub version: String,
    pub model: String,
    pub provider: String,
    pub sandbox: SandboxStatus,
    pub shortcuts: Vec<Shortcut>,
}

impl Default for BannerInfo {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            model: "unknown".to_string(),
            provider: "unknown".to_string(),
            sandbox: SandboxStatus::default(),
            shortcuts: DEFAULT_SHORTCUTS.to_vec(),
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

/// Number of rows [`render`] needs at a given width, including the border.
pub fn height_for(info: &BannerInfo, width: u16) -> u16 {
    let inner_width = width.saturating_sub(2);
    let logo = logo_lines(LogoSize::for_width(inner_width)).len() as u16;
    let hints = shortcut_lines(&info.shortcuts, inner_width).len() as u16;
    // logo + status + hints + top/bottom border
    logo + 1 + hints + 2
}

/// Draw the banner into `area` as a bordered welcome panel.
pub fn render(frame: &mut Frame<'_>, area: Rect, info: &BannerInfo) {
    let inner_width = area.width.saturating_sub(2);
    let mut lines: Vec<Line<'_>> = logo_lines(LogoSize::for_width(inner_width))
        .iter()
        .map(|row| {
            Line::from(Span::styled(
                *row,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();

    lines.push(Line::from(vec![
        Span::styled(
            format!("v{}", info.version),
            Style::default().fg(Color::White),
        ),
        Span::raw(" · "),
        Span::styled(
            format!("{}/{}", info.provider, info.model),
            Style::default().fg(Color::Magenta),
        ),
        Span::raw(" · "),
        Span::raw("sandbox: "),
        Span::styled(
            info.sandbox.label(),
            Style::default()
                .fg(info.sandbox.color())
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    for hint in shortcut_lines(&info.shortcuts, inner_width) {
        lines.push(Line::from(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        )));
    }

    let panel = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Welcome "));
    frame.render_widget(panel, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> BannerInfo {
        BannerInfo {
            version: "0.0.0".to_string(),
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            sandbox: SandboxStatus::Workspace,
            shortcuts: DEFAULT_SHORTCUTS.to_vec(),
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
    fn height_matches_rendered_line_count() {
        let info = info();
        for width in [20u16, 40, 80] {
            let rows = render_text(&info, width.saturating_sub(2))
                .lines()
                .filter(|l| !l.is_empty())
                .count() as u16;
            assert_eq!(height_for(&info, width), rows + 2);
        }
    }
}
