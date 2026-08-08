//! The shared color palette for `cust-tui`'s inline surface.
//!
//! Green is the brand accent: logo, panel borders, headers, and highlighted
//! status-line segments all draw from [`PRIMARY`]/[`PRIMARY_DIM`]. Red/yellow
//! stay reserved for actual danger/caution states (sandbox off, bypass
//! permissions, high context usage) — those are semantic, not brand, and
//! recoloring them green would erase the one signal a user glances at the
//! status line for.

/// Reset all SGR attributes.
pub const RESET: &str = "\u{1b}[0m";
/// Bold/increased intensity.
pub const BOLD: &str = "\u{1b}[1m";
/// Faint/decreased intensity, used for secondary text (hints, tips, paths).
pub const DIM: &str = "\u{1b}[2m";

/// Brand accent — logo, panel borders/titles, primary status-line segments.
pub const PRIMARY_RGB: (u8, u8, u8) = (0, 200, 83);
/// A darker shade of the brand color, for content that needs contrast
/// against a [`PRIMARY_RGB`]-filled background (the mascot's eyes) rather
/// than a separate hue.
pub const PRIMARY_DARK_RGB: (u8, u8, u8) = (0, 62, 28);

/// Danger — sandbox off, bypass-permissions mode, context over budget.
pub const DANGER: &str = "\u{1b}[31m";
/// Caution — workspace-write sandbox, accept-edits permission mode.
pub const CAUTION: &str = "\u{1b}[33m";
/// Neutral/inactive — ask/plan permission modes, separators.
pub const NEUTRAL: &str = "\u{1b}[90m";

/// 24-bit foreground escape for an arbitrary RGB triple.
pub fn fg_rgb((r, g, b): (u8, u8, u8)) -> String {
    format!("\u{1b}[38;2;{r};{g};{b}m")
}

/// 24-bit background escape for an arbitrary RGB triple.
pub fn bg_rgb((r, g, b): (u8, u8, u8)) -> String {
    format!("\u{1b}[48;2;{r};{g};{b}m")
}

/// The brand accent as a foreground escape — shorthand for the common case.
pub fn primary() -> String {
    fg_rgb(PRIMARY_RGB)
}

/// [`primary`] wrapped so the following text is also bold, since most brand
/// highlights (headers, model names) want both.
pub fn primary_bold(text: &str) -> String {
    format!("{}{BOLD}{text}{RESET}", primary())
}

/// [`primary`] as a dim/secondary variant, for de-emphasized brand text
/// (version numbers, workspace paths) that still shouldn't be red or yellow.
pub fn primary_dim(text: &str) -> String {
    format!("{}{DIM}{text}{RESET}", primary())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fg_rgb_formats_a_truecolor_escape() {
        assert_eq!(fg_rgb((0, 200, 83)), "\u{1b}[38;2;0;200;83m");
    }

    #[test]
    fn bg_rgb_formats_a_truecolor_escape() {
        assert_eq!(bg_rgb((0, 200, 83)), "\u{1b}[48;2;0;200;83m");
    }

    #[test]
    fn primary_bold_wraps_and_resets() {
        let s = primary_bold("hi");
        assert!(s.starts_with(&primary()));
        assert!(s.contains(BOLD));
        assert!(s.ends_with(RESET));
        assert!(s.contains("hi"));
    }

    #[test]
    fn primary_dim_wraps_and_resets() {
        let s = primary_dim("v1.0");
        assert!(s.starts_with(&primary()));
        assert!(s.contains(DIM));
        assert!(s.ends_with(RESET));
    }
}
