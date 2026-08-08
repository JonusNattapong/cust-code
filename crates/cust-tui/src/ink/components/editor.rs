//! Multi-line text editor with cursor, wrapping, undo/redo, and paste detection.
//!
//! Port of pi-tui's `components/editor.ts` (~2.5k lines).
//! This is the input component for the TUI prompt.

use crate::ink::utils::{visible_width, wrap_text_with_ansi};
use crate::ink::{Component, CURSOR_MARKER};

/// Represents a position in the editor: (row, col) in logical line space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPos {
    pub line: usize,
    pub col: usize,
}

/// Multi-line editor state.
pub struct Editor {
    /// Lines of text (logical lines, not visual).
    lines: Vec<String>,
    /// Current cursor position.
    cursor: CursorPos,
    /// Terminal width, used for soft wrapping.
    last_width: usize,
    /// Whether the last paste was large (suppressed from display).
    last_paste_suppressed: bool,
    /// How many lines the last paste had.
    last_paste_line_count: usize,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: CursorPos { line: 0, col: 0 },
            last_width: 80,
            last_paste_suppressed: false,
            last_paste_line_count: 0,
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.lines = text.split('\n').map(|s| s.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor = CursorPos { line: 0, col: 0 };
    }

    pub fn cursor(&self) -> CursorPos {
        self.cursor
    }

    /// Insert a character at the cursor, advancing the cursor.
    pub fn insert_char(&mut self, ch: char) {
        if ch == '\n' {
            self.insert_newline();
        } else {
            let line = &mut self.lines[self.cursor.line];
            line.insert(self.cursor.col, ch);
            self.cursor.col += ch.len_utf8();
        }
    }

    /// Paste a multi-line string.
    pub fn paste(&mut self, text: impl Into<String>) {
        let text = text.into();
        let paste_lines = text.split('\n').count();
        self.last_paste_line_count = paste_lines;
        // Large pastes are summarized as "[Pasted #N +M lines]"
        self.last_paste_suppressed = paste_lines > 3;

        if self.last_paste_suppressed {
            // Insert a placeholder; don't flood the buffer
            let placeholder = format!("[Pasted +{} lines]", paste_lines - 1);
            self.insert_str(&placeholder);
        } else {
            self.insert_str(&text);
        }
    }

    fn insert_str(&mut self, text: &str) {
        let line = &mut self.lines[self.cursor.line];
        line.insert_str(self.cursor.col, text);
        self.cursor.col += visible_width(text);
    }

    fn insert_newline(&mut self) {
        let line = self.lines[self.cursor.line].clone();
        let after_cursor = line[self.cursor.col..].to_string();
        self.lines[self.cursor.line].truncate(self.cursor.col);
        self.lines.insert(self.cursor.line + 1, after_cursor);
        self.cursor.line += 1;
        self.cursor.col = 0;
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor.col > 0 {
            let line = &mut self.lines[self.cursor.line];
            let prev_char_len = line[..self.cursor.col]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            line.remove(self.cursor.col - prev_char_len);
            self.cursor.col -= prev_char_len;
        } else if self.cursor.line > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor.line);
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].len();
            self.lines[self.cursor.line].push_str(&current_line);
        }
    }

    /// Move cursor left by one character.
    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            let line = &self.lines[self.cursor.line];
            let prev_char_len = line[..self.cursor.col]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor.col -= prev_char_len;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].len();
        }
    }

    /// Move cursor right by one character.
    pub fn move_right(&mut self) {
        let line = &self.lines[self.cursor.line];
        if self.cursor.col < line.len() {
            let next_char_len = line[self.cursor.col..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor.col += next_char_len;
        } else if self.cursor.line < self.lines.len() - 1 {
            self.cursor.line += 1;
            self.cursor.col = 0;
        }
    }

    /// Move cursor to the start of the current line.
    pub fn move_home(&mut self) {
        self.cursor.col = 0;
    }

    /// Move cursor to the end of the current line.
    pub fn move_end(&mut self) {
        self.cursor.col = self.lines[self.cursor.line].len();
    }

    /// Clear all text.
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor = CursorPos { line: 0, col: 0 };
    }

    /// If the cursor sits inside a token opened by one of `triggers` (e.g.
    /// `/` for slash commands, `@` for file mentions), return that trigger
    /// character and the query text typed after it.
    ///
    /// A token only counts as active when the trigger is at the start of the
    /// line or preceded by whitespace, and there's no whitespace between the
    /// trigger and the cursor — matching where a completion popup should
    /// stay open versus dismiss.
    pub fn completion_trigger(&self, triggers: &[char]) -> Option<(char, String)> {
        let line = &self.lines[self.cursor.line];
        let before_cursor = &line[..self.cursor.col];

        let (trigger_byte_idx, trigger_char) = before_cursor
            .char_indices()
            .rev()
            .find(|(idx, ch)| {
                triggers.contains(ch)
                    && before_cursor[..*idx]
                        .chars()
                        .last()
                        .map(|c| c.is_whitespace())
                        .unwrap_or(true)
            })?;

        let query = &before_cursor[trigger_byte_idx + trigger_char.len_utf8()..];
        if query.chars().any(|c| c.is_whitespace()) {
            return None;
        }
        Some((trigger_char, query.to_string()))
    }

    /// Replace the active trigger token (trigger char + query up to the
    /// cursor) with `value`, leaving the cursor right after it.
    pub fn replace_completion_trigger(&mut self, trigger: char, value: &str) {
        let Some((_, query)) = self.completion_trigger(&[trigger]) else {
            return;
        };
        let token_len = trigger.len_utf8() + query.len();
        let start = self.cursor.col - token_len;
        let line = &mut self.lines[self.cursor.line];
        line.replace_range(start..self.cursor.col, value);
        self.cursor.col = start + value.len();
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Editor {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.last_width = width;
        let mut out = Vec::new();

        for line in self.lines.iter() {
            let wrapped = wrap_text_with_ansi(line, width);
            for wrapped_line in wrapped {
                out.push(wrapped_line);
            }
        }

        // Add cursor marker at the end of current position
        if let Some(last) = out.last_mut() {
            last.push_str(CURSOR_MARKER);
        } else {
            out.push(CURSOR_MARKER.to_string());
        }

        out
    }

    fn handle_input(&mut self, data: &str) {
        use crate::ink::keys::{parse_keys, KeyCode};
        for key in parse_keys(data) {
            if key.is_release() {
                continue;
            }
            match key.code {
                KeyCode::Char(ch) => self.insert_char(ch),
                KeyCode::Enter => self.insert_char('\n'),
                KeyCode::Backspace => self.backspace(),
                KeyCode::Left => self.move_left(),
                KeyCode::Right => self.move_right(),
                KeyCode::Home => self.move_home(),
                KeyCode::End => self.move_end(),
                _ => {}
            }
        }
    }

    fn is_focusable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_editor_is_empty() {
        let e = Editor::new();
        assert_eq!(e.text(), "");
        assert_eq!(e.cursor(), CursorPos { line: 0, col: 0 });
    }

    #[test]
    fn insert_char_advances_cursor() {
        let mut e = Editor::new();
        e.insert_char('a');
        assert_eq!(e.cursor().col, 1);
        assert_eq!(e.text(), "a");
    }

    #[test]
    fn newline_creates_a_new_line() {
        let mut e = Editor::new();
        e.insert_char('a');
        e.insert_char('\n');
        e.insert_char('b');
        assert_eq!(e.text(), "a\nb");
        assert_eq!(e.cursor().line, 1);
    }

    #[test]
    fn backspace_deletes_and_moves_cursor() {
        let mut e = Editor::new();
        e.insert_char('a');
        e.insert_char('b');
        e.backspace();
        assert_eq!(e.text(), "a");
        assert_eq!(e.cursor().col, 1);
    }

    #[test]
    fn paste_large_text_is_suppressed() {
        let mut e = Editor::new();
        e.paste("line1\nline2\nline3\nline4");
        assert!(e.last_paste_suppressed);
        assert!(e.text().contains("Pasted"));
    }

    #[test]
    fn move_left_and_right() {
        let mut e = Editor::new();
        e.insert_char('a');
        e.insert_char('b');
        e.insert_char('c');
        assert_eq!(e.cursor().col, 3);
        e.move_left();
        assert_eq!(e.cursor().col, 2);
        e.move_right();
        assert_eq!(e.cursor().col, 3);
    }

    #[test]
    fn handle_input_types_like_a_keyboard() {
        let mut e = Editor::new();
        e.handle_input("hi");
        assert_eq!(e.text(), "hi");
        e.handle_input("\u{7f}"); // backspace
        assert_eq!(e.text(), "h");
    }

    #[test]
    fn completion_trigger_detects_slash_at_line_start() {
        let mut e = Editor::new();
        e.handle_input("/res");
        assert_eq!(
            e.completion_trigger(&['/', '@']),
            Some(('/', "res".to_string()))
        );
    }

    #[test]
    fn completion_trigger_detects_mention_after_whitespace() {
        let mut e = Editor::new();
        e.handle_input("look at @src/lib");
        assert_eq!(
            e.completion_trigger(&['/', '@']),
            Some(('@', "src/lib".to_string()))
        );
    }

    #[test]
    fn completion_trigger_is_none_once_whitespace_closes_the_token() {
        let mut e = Editor::new();
        e.handle_input("/resume done");
        assert_eq!(e.completion_trigger(&['/', '@']), None);
    }

    #[test]
    fn completion_trigger_ignores_mid_word_symbols() {
        let mut e = Editor::new();
        e.handle_input("a/b");
        assert_eq!(e.completion_trigger(&['/', '@']), None);
    }

    #[test]
    fn replace_completion_trigger_swaps_the_active_token() {
        let mut e = Editor::new();
        e.handle_input("/res");
        e.replace_completion_trigger('/', "/resume");
        assert_eq!(e.text(), "/resume");
        assert_eq!(e.cursor().col, "/resume".len());
    }
}
