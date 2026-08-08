use super::text::BgFn;
use crate::ink::utils::{apply_background_to_line, pad_to_width, visible_width};
use crate::ink::Component;

/// A container that pads its children and paints a shared background.
///
/// Port of `pi-tui`'s `Box` (renamed to avoid colliding with `std::boxed::Box`).
/// With no children it renders nothing — an empty box should not reserve rows.
#[derive(Default)]
pub struct BoxView {
    children: Vec<std::boxed::Box<dyn Component>>,
    padding_x: usize,
    padding_y: usize,
    bg: Option<BgFn>,
    border: Option<BorderStyle>,
    /// ANSI escape prefix applied to the border and title; `None` leaves
    /// them in the terminal's default foreground.
    border_color: Option<String>,
    title: Option<String>,
    cache: Option<(usize, Vec<String>, Vec<String>)>,
}

const BOX_ANSI_RESET: &str = "\u{1b}[0m";

/// Box-drawing characters for a bordered box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    Single,
    Rounded,
    Double,
}

impl BorderStyle {
    /// (top-left, top-right, bottom-left, bottom-right, horizontal, vertical)
    fn chars(self) -> (char, char, char, char, char, char) {
        match self {
            BorderStyle::Single => ('┌', '┐', '└', '┘', '─', '│'),
            BorderStyle::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
            BorderStyle::Double => ('╔', '╗', '╚', '╝', '═', '║'),
        }
    }
}

impl BoxView {
    pub fn new() -> Self {
        Self {
            padding_x: 1,
            padding_y: 1,
            ..Default::default()
        }
    }

    pub fn with_padding(mut self, x: usize, y: usize) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self.cache = None;
        self
    }

    pub fn with_background(mut self, bg: BgFn) -> Self {
        self.bg = Some(bg);
        self.cache = None;
        self
    }

    /// Draw a border around the box. Borders consume one cell on each side.
    pub fn with_border(mut self, style: BorderStyle) -> Self {
        self.border = Some(style);
        self.cache = None;
        self
    }

    /// Title inlaid into the top border. Ignored when there is no border.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self.cache = None;
        self
    }

    /// Color the border (and title) with a raw ANSI escape prefix, e.g.
    /// `"\x1b[38;2;0;200;83m"`. Ignored when there is no border.
    pub fn with_border_color(mut self, ansi_prefix: impl Into<String>) -> Self {
        self.border_color = Some(ansi_prefix.into());
        self.cache = None;
        self
    }

    pub fn add_child(&mut self, child: std::boxed::Box<dyn Component>) {
        self.children.push(child);
        self.cache = None;
    }

    pub fn clear(&mut self) {
        self.children.clear();
        self.cache = None;
    }

    fn paint(&self, line: &str, width: usize) -> String {
        match &self.bg {
            Some(bg) => apply_background_to_line(line, width, bg.as_ref()),
            None => pad_to_width(line, width),
        }
    }

    /// Build the top border, inlaying the title if it fits.
    fn top_border(&self, style: BorderStyle, width: usize) -> String {
        let (tl, tr, _, _, h, _) = style.chars();
        let inner = width.saturating_sub(2);
        let plain = match &self.title {
            // `┌─ title ─...─┐` needs 3 cells of framing around the title.
            Some(t) if visible_width(t) + 3 <= inner => {
                let label = format!("{h} {t} ");
                let fill = inner - visible_width(&label);
                format!("{tl}{label}{}{tr}", h.to_string().repeat(fill))
            }
            _ => format!("{tl}{}{tr}", h.to_string().repeat(inner)),
        };
        self.colorize(&plain)
    }

    fn colorize(&self, s: &str) -> String {
        match &self.border_color {
            Some(prefix) => format!("{prefix}{s}{BOX_ANSI_RESET}"),
            None => s.to_string(),
        }
    }
}

impl Component for BoxView {
    fn render(&mut self, width: usize) -> Vec<String> {
        if self.children.is_empty() {
            self.cache = None;
            return Vec::new();
        }

        let border_inset = if self.border.is_some() { 1 } else { 0 };
        let frame = (self.padding_x + border_inset) * 2;
        let content_width = width.saturating_sub(frame).max(1);
        let left_pad = " ".repeat(self.padding_x);

        let mut child_lines = Vec::new();
        for child in &mut self.children {
            for line in child.render(content_width) {
                child_lines.push(format!("{left_pad}{line}"));
            }
        }
        if child_lines.is_empty() {
            self.cache = None;
            return Vec::new();
        }

        if let Some((cached_width, cached_children, lines)) = &self.cache {
            if *cached_width == width && cached_children == &child_lines {
                return lines.clone();
            }
        }

        let body_width = width.saturating_sub(border_inset * 2);
        let mut body = Vec::new();
        for _ in 0..self.padding_y {
            body.push(self.paint("", body_width));
        }
        for line in &child_lines {
            body.push(self.paint(line, body_width));
        }
        for _ in 0..self.padding_y {
            body.push(self.paint("", body_width));
        }

        let result = match self.border {
            None => body,
            Some(style) => {
                let (_, _, bl, br, h, v) = style.chars();
                let side = self.colorize(&v.to_string());
                let mut out = vec![self.top_border(style, width)];
                for line in body {
                    out.push(format!("{side}{line}{side}"));
                }
                out.push(self.colorize(&format!(
                    "{bl}{}{br}",
                    h.to_string().repeat(width.saturating_sub(2))
                )));
                out
            }
        };

        self.cache = Some((width, child_lines, result.clone()));
        result
    }

    fn invalidate(&mut self) {
        self.cache = None;
        for child in &mut self.children {
            child.invalidate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::components::Text;

    fn text_child(s: &str) -> std::boxed::Box<dyn Component> {
        std::boxed::Box::new(Text::new(s).with_padding(0, 0))
    }

    #[test]
    fn empty_box_renders_nothing() {
        assert!(BoxView::new().render(20).is_empty());
    }

    #[test]
    fn box_with_only_blank_children_renders_nothing() {
        let mut b = BoxView::new();
        b.add_child(text_child(""));
        assert!(b.render(20).is_empty());
    }

    #[test]
    fn pads_children_and_fills_width() {
        let mut b = BoxView::new().with_padding(2, 1);
        b.add_child(text_child("hi"));
        let lines = b.render(20);
        assert_eq!(lines.len(), 3);
        assert!(lines[1].starts_with("  hi"));
        for line in &lines {
            assert_eq!(visible_width(line), 20);
        }
    }

    #[test]
    fn border_frames_the_content() {
        let mut b = BoxView::new().with_padding(0, 0).with_border(BorderStyle::Single);
        b.add_child(text_child("hi"));
        let lines = b.render(10);
        assert!(lines[0].starts_with('┌') && lines[0].ends_with('┐'));
        assert!(lines[1].starts_with('│') && lines[1].ends_with('│'));
        assert!(lines[2].starts_with('└') && lines[2].ends_with('┘'));
        for line in &lines {
            assert_eq!(visible_width(line), 10);
        }
    }

    #[test]
    fn title_is_inlaid_into_the_top_border() {
        let mut b = BoxView::new().with_padding(0, 0).with_border(BorderStyle::Rounded).with_title("Log");
        b.add_child(text_child("hi"));
        let lines = b.render(20);
        assert!(lines[0].contains("Log"));
        assert_eq!(visible_width(&lines[0]), 20);
    }

    #[test]
    fn title_too_long_falls_back_to_a_plain_border() {
        let mut b = BoxView::new()
            .with_padding(0, 0)
            .with_border(BorderStyle::Single)
            .with_title("a-very-long-title");
        b.add_child(text_child("hi"));
        let lines = b.render(10);
        assert!(!lines[0].contains("a-very"));
        assert_eq!(visible_width(&lines[0]), 10);
    }

    #[test]
    fn children_render_against_the_inner_width() {
        // Border (2) + padding (2) leaves 6 columns, forcing a wrap.
        let mut b = BoxView::new().with_padding(1, 0).with_border(BorderStyle::Single);
        b.add_child(text_child("aaa bbb"));
        let lines = b.render(10);
        assert_eq!(lines.len(), 4); // top + 2 content + bottom
    }

    #[test]
    fn border_color_wraps_border_chars_and_leaves_content_alone() {
        let mut b = BoxView::new()
            .with_padding(0, 0)
            .with_border(BorderStyle::Single)
            .with_title("Log")
            .with_border_color("\u{1b}[38;2;0;200;83m");
        b.add_child(text_child("hi"));
        let lines = b.render(20);
        assert!(lines[0].starts_with("\u{1b}[38;2;0;200;83m"));
        assert!(lines[0].ends_with("\u{1b}[0m"));
        // The content row's escape sequence brackets only the border char,
        // not the text between them.
        assert!(lines[1].contains("\u{1b}[38;2;0;200;83m\u{2502}\u{1b}[0mhi"));
        assert!(lines[2].starts_with("\u{1b}[38;2;0;200;83m"));
    }

    #[test]
    fn no_border_color_leaves_border_chars_plain() {
        let mut b = BoxView::new().with_padding(0, 0).with_border(BorderStyle::Single);
        b.add_child(text_child("hi"));
        let lines = b.render(10);
        assert!(!lines[0].contains('\u{1b}'));
    }
}
