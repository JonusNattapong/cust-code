use crate::ink::utils::truncate_to_width;
use crate::ink::Component;

/// Single-line text clipped to the render width. Port of `pi-tui`'s
/// `TruncatedText`.
///
/// Unlike [`super::Text`] it never wraps: overflow is cut and marked with an
/// ellipsis, which is what status lines and list rows want.
#[derive(Debug, Clone)]
pub struct TruncatedText {
    text: String,
    ellipsis: String,
    padding_x: usize,
}

impl TruncatedText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ellipsis: "…".to_string(),
            padding_x: 0,
        }
    }

    pub fn with_ellipsis(mut self, ellipsis: impl Into<String>) -> Self {
        self.ellipsis = ellipsis.into();
        self
    }

    pub fn with_padding_x(mut self, padding_x: usize) -> Self {
        self.padding_x = padding_x;
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

impl Component for TruncatedText {
    fn render(&mut self, width: usize) -> Vec<String> {
        if self.text.is_empty() {
            return Vec::new();
        }
        let pad = " ".repeat(self.padding_x);
        let content_width = width.saturating_sub(self.padding_x * 2);
        if content_width == 0 {
            return vec![String::new()];
        }
        // Only the first line survives; embedded newlines would break the
        // one-row contract callers rely on.
        let first_line = self.text.split('\n').next().unwrap_or_default();
        let clipped = truncate_to_width(first_line, content_width, Some(&self.ellipsis));
        vec![format!("{pad}{}{pad}", clipped.text)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::utils::visible_width;

    #[test]
    fn short_text_passes_through() {
        assert_eq!(TruncatedText::new("hi").render(20), vec!["hi"]);
    }

    #[test]
    fn long_text_is_clipped_with_an_ellipsis() {
        let out = TruncatedText::new("hello world").render(8);
        assert_eq!(visible_width(&out[0]), 8);
        assert!(out[0].ends_with('…'));
    }

    #[test]
    fn only_the_first_line_is_rendered() {
        assert_eq!(TruncatedText::new("a\nb").render(20), vec!["a"]);
    }

    #[test]
    fn empty_text_renders_nothing() {
        assert!(TruncatedText::new("").render(20).is_empty());
    }

    #[test]
    fn padding_shrinks_the_content_budget() {
        let out = TruncatedText::new("hello world").with_padding_x(2).render(10);
        assert!(out[0].starts_with("  "));
        assert_eq!(visible_width(&out[0]), 10);
    }

    #[test]
    fn zero_content_width_yields_a_blank_row() {
        assert_eq!(TruncatedText::new("hi").with_padding_x(5).render(4), vec![""]);
    }
}
