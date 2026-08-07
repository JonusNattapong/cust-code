use crate::ink::utils::{apply_background_to_line, pad_to_width, wrap_text_with_ansi};
use crate::ink::Component;

/// A background painter: takes a row's text and returns it wrapped in styling.
pub type BgFn = Box<dyn Fn(&str) -> String + Send + Sync>;

/// Word-wrapped multi-line text with padding. Port of `pi-tui`'s `Text`.
///
/// Output rows are always padded to the full render width so a background fill
/// reaches the right edge instead of stopping at the last character.
pub struct Text {
    text: String,
    padding_x: usize,
    padding_y: usize,
    bg: Option<BgFn>,
    cache: Option<(String, usize, Vec<String>)>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            padding_x: 1,
            padding_y: 1,
            bg: None,
            cache: None,
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

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cache = None;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    fn paint(&self, line: &str, width: usize) -> String {
        match &self.bg {
            Some(bg) => apply_background_to_line(line, width, bg.as_ref()),
            None => pad_to_width(line, width),
        }
    }
}

impl Component for Text {
    fn render(&mut self, width: usize) -> Vec<String> {
        if let Some((cached_text, cached_width, lines)) = &self.cache {
            if cached_text == &self.text && *cached_width == width {
                return lines.clone();
            }
        }

        // Whitespace-only text renders nothing at all — not even padding —
        // so empty slots collapse instead of leaving gaps in a transcript.
        if self.text.trim().is_empty() {
            self.cache = Some((self.text.clone(), width, Vec::new()));
            return Vec::new();
        }

        let normalized = self.text.replace('\t', "   ");
        let content_width = width.saturating_sub(self.padding_x * 2).max(1);
        let pad = " ".repeat(self.padding_x);

        let mut result = Vec::new();
        let blank = self.paint(&" ".repeat(width), width);
        for _ in 0..self.padding_y {
            result.push(blank.clone());
        }
        for line in wrap_text_with_ansi(&normalized, content_width) {
            result.push(self.paint(&format!("{pad}{line}{pad}"), width));
        }
        for _ in 0..self.padding_y {
            result.push(blank.clone());
        }

        self.cache = Some((self.text.clone(), width, result.clone()));
        result
    }

    fn invalidate(&mut self) {
        self.cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::utils::visible_width;

    #[test]
    fn empty_text_renders_nothing() {
        assert!(Text::new("").render(20).is_empty());
        assert!(Text::new("   \n  ").render(20).is_empty());
    }

    #[test]
    fn pads_every_row_to_full_width() {
        let mut t = Text::new("hi").with_padding(1, 1);
        for line in t.render(20) {
            assert_eq!(visible_width(&line), 20);
        }
    }

    #[test]
    fn applies_vertical_padding_around_content() {
        let mut t = Text::new("hi").with_padding(1, 2);
        let lines = t.render(20);
        assert_eq!(lines.len(), 5); // 2 blank + 1 content + 2 blank
        assert!(lines[2].contains("hi"));
    }

    #[test]
    fn wraps_within_horizontal_padding() {
        let mut t = Text::new("aaa bbb ccc").with_padding(2, 0);
        let lines = t.render(9); // content width 5
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("  aaa"));
    }

    #[test]
    fn tabs_become_three_spaces() {
        let mut t = Text::new("a\tb").with_padding(0, 0);
        assert!(t.render(20)[0].starts_with("a   b"));
    }

    #[test]
    fn reuses_cache_until_text_or_width_changes() {
        let mut t = Text::new("hi").with_padding(0, 0);
        let a = t.render(20);
        assert_eq!(a, t.render(20));
        assert_ne!(a, t.render(10));
        t.set_text("bye");
        assert_ne!(a, t.render(20));
    }

    #[test]
    fn background_covers_the_padding() {
        let mut t = Text::new("hi")
            .with_padding(0, 0)
            .with_background(Box::new(|s| format!("<{s}>")));
        let lines = t.render(10);
        assert!(lines[0].starts_with('<') && lines[0].ends_with('>'));
        assert!(lines[0].contains("hi        "));
    }
}
