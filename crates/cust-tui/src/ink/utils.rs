//! ANSI-aware text measurement, wrapping and slicing.
//!
//! Port of `pi-tui`'s `src/utils.ts` (prime-agent). The invariant everything
//! else depends on: a "line" is a `String` that may contain SGR/OSC escape
//! sequences, and its *visible* width is the number of terminal cells it
//! occupies, not its byte or char count.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A parsed escape sequence starting at some byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiCode {
    /// The full sequence including the introducer.
    pub text: String,
    /// Byte offset just past the sequence.
    pub end: usize,
}

const ESC: char = '\u{1b}';
const BEL: char = '\u{7}';

/// Extract the escape sequence beginning at byte offset `pos`, if any.
///
/// Handles CSI (`ESC [ ... final`), OSC / DCS / APC / PM / SOS control strings
/// (terminated by BEL or ST), and short two-character sequences.
pub fn extract_ansi_code(s: &str, pos: usize) -> Option<AnsiCode> {
    let rest = s.get(pos..)?;
    let mut chars = rest.char_indices();
    if chars.next()?.1 != ESC {
        return None;
    }
    let (_, second) = chars.next()?;

    let end = match second {
        // CSI: parameters/intermediates then a final byte in @..~
        '[' => {
            let mut end = None;
            for (i, c) in rest.char_indices().skip(2) {
                if ('\u{40}'..='\u{7e}').contains(&c) {
                    end = Some(i + c.len_utf8());
                    break;
                }
            }
            end?
        }
        // Control strings: consume until BEL or ST (ESC \).
        ']' | 'P' | '_' | '^' | 'X' => {
            let mut end = None;
            let mut prev_esc = false;
            for (i, c) in rest.char_indices().skip(2) {
                if c == BEL {
                    end = Some(i + c.len_utf8());
                    break;
                }
                if prev_esc && c == '\\' {
                    end = Some(i + c.len_utf8());
                    break;
                }
                prev_esc = c == ESC;
            }
            // Unterminated control string: treat the remainder as the sequence.
            end.unwrap_or(rest.len())
        }
        // Two-character sequence (ESC 7, ESC =, ...).
        other => 1 + other.len_utf8(),
    };

    Some(AnsiCode {
        text: rest[..end].to_string(),
        end: pos + end,
    })
}

/// Remove every escape sequence, leaving only printable content.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if let Some(code) = extract_ansi_code(s, i) {
            i = code.end;
            continue;
        }
        let c = s[i..].chars().next().expect("byte index is a char boundary");
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Width in terminal cells of a single grapheme cluster.
///
/// `unicode-width` measures per code point, which gets emoji sequences (ZWJ
/// joins, variation selectors, flags, skin-tone modifiers) wrong by summing
/// their parts. Any cluster that carries emoji machinery is forced to 2.
pub fn grapheme_width(cluster: &str) -> usize {
    if cluster.is_empty() {
        return 0;
    }
    let mut has_emoji_presentation = false;
    let mut has_text_presentation = false;
    for c in cluster.chars() {
        match c {
            // Variation selectors pick emoji (FE0F) or text (FE0E) rendering.
            '\u{fe0f}' => has_emoji_presentation = true,
            '\u{fe0e}' => has_text_presentation = true,
            // ZWJ sequences, skin tones, regional indicators (flags), keycaps.
            '\u{200d}' | '\u{20e3}' => has_emoji_presentation = true,
            '\u{1f3fb}'..='\u{1f3ff}' => has_emoji_presentation = true,
            '\u{1f1e6}'..='\u{1f1ff}' => has_emoji_presentation = true,
            _ => {}
        }
    }
    if has_emoji_presentation && !has_text_presentation {
        return 2;
    }
    // Combining marks and other zero-width joiners collapse into the base.
    UnicodeWidthStr::width(cluster).max(if has_text_presentation { 1 } else { 0 })
}

/// Visible width of a string in terminal cells, ignoring escape sequences.
pub fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut i = 0;
    let mut plain = String::with_capacity(s.len());
    while i < s.len() {
        if let Some(code) = extract_ansi_code(s, i) {
            i = code.end;
            continue;
        }
        let c = s[i..].chars().next().expect("byte index is a char boundary");
        plain.push(c);
        i += c.len_utf8();
    }
    for cluster in plain.graphemes(true) {
        width += grapheme_width(cluster);
    }
    width
}

/// Result of truncating: the (possibly shortened) text and its visible width.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Truncated {
    pub text: String,
    pub width: usize,
}

/// Truncate `s` to at most `max_width` visible cells, preserving escape
/// sequences so styling is never left half-open.
///
/// When `ellipsis` is set and truncation actually happened, it is appended and
/// the content is shortened to make room for it.
pub fn truncate_to_width(s: &str, max_width: usize, ellipsis: Option<&str>) -> Truncated {
    let full_width = visible_width(s);
    if full_width <= max_width {
        return Truncated {
            text: s.to_string(),
            width: full_width,
        };
    }

    let suffix = ellipsis.unwrap_or("");
    let suffix_width = visible_width(suffix);
    // Not even the ellipsis fits: fall back to a hard cut with no suffix.
    let budget = max_width.saturating_sub(suffix_width);
    let (mut out, width) = take_cells(s, budget);
    if suffix_width <= max_width {
        out.push_str(suffix);
        return Truncated {
            text: out,
            width: width + suffix_width,
        };
    }
    let (out, width) = take_cells(s, max_width);
    Truncated { text: out, width }
}

/// Consume graphemes until `budget` cells are used, carrying escape sequences
/// through verbatim (they cost nothing) and never splitting a wide cluster.
fn take_cells(s: &str, budget: usize) -> (String, usize) {
    let mut out = String::new();
    let mut used = 0;
    let mut i = 0;
    while i < s.len() {
        if let Some(code) = extract_ansi_code(s, i) {
            out.push_str(&code.text);
            i = code.end;
            continue;
        }
        // Take the next grapheme cluster starting here.
        let cluster = match s[i..].graphemes(true).next() {
            Some(c) => c,
            None => break,
        };
        let w = grapheme_width(cluster);
        if used + w > budget {
            break;
        }
        out.push_str(cluster);
        used += w;
        i += cluster.len();
    }
    (out, used)
}

/// Extract the substring covering visible columns `[start_col, start_col + len)`.
///
/// Escape sequences preceding the window are preserved at the front so the
/// slice renders with the same styling it had in place.
pub fn slice_by_column(line: &str, start_col: usize, len: usize) -> String {
    let mut prefix_codes = String::new();
    let mut out = String::new();
    let mut col = 0;
    let mut i = 0;
    while i < line.len() {
        if let Some(code) = extract_ansi_code(line, i) {
            if col <= start_col {
                prefix_codes.push_str(&code.text);
            } else {
                out.push_str(&code.text);
            }
            i = code.end;
            continue;
        }
        let cluster = match line[i..].graphemes(true).next() {
            Some(c) => c,
            None => break,
        };
        let w = grapheme_width(cluster);
        if col >= start_col + len {
            break;
        }
        if col >= start_col {
            out.push_str(cluster);
        }
        col += w;
        i += cluster.len();
    }
    format!("{prefix_codes}{out}")
}

/// True for characters that may be collapsed when wrapping.
pub fn is_whitespace_char(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// Tracks which SGR codes are active so wrapped continuation lines can reopen
/// the styling the break interrupted.
#[derive(Debug, Default, Clone)]
struct AnsiTracker {
    active: Vec<String>,
}

impl AnsiTracker {
    fn observe(&mut self, code: &str) {
        // Only SGR sequences (CSI ... m) carry state worth reopening.
        if !code.starts_with("\u{1b}[") || !code.ends_with('m') {
            return;
        }
        let params = &code[2..code.len() - 1];
        // A bare reset (ESC[m or ESC[0m) clears everything.
        if params.is_empty() || params.split(';').all(|p| p == "0" || p.is_empty()) {
            self.active.clear();
            return;
        }
        self.active.push(code.to_string());
    }

    fn prefix(&self) -> String {
        self.active.concat()
    }

    fn suffix(&self) -> &'static str {
        if self.active.is_empty() {
            ""
        } else {
            "\u{1b}[0m"
        }
    }
}

/// Word-wrap `text` to `width` cells, preserving ANSI styling across breaks.
///
/// Explicit newlines are honoured. Words longer than `width` are hard-broken.
/// Returns at least one line (possibly empty) for a non-empty input.
pub fn wrap_text_with_ansi(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for line in text.split('\n') {
        out.extend(wrap_single_line(line, width));
    }
    out
}

fn wrap_single_line(line: &str, width: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    if visible_width(line) <= width {
        return vec![line.to_string()];
    }

    let mut tracker = AnsiTracker::default();
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    // Flush `current` as a completed line, reopening styles on the next one.
    macro_rules! flush {
        () => {{
            if !current.is_empty() || !lines.is_empty() {
                // A space that lands on the break point is what separated the
                // two words; it must not survive as trailing padding.
                let mut done = std::mem::take(&mut current).trim_end().to_string();
                done.push_str(tracker.suffix());
                lines.push(done);
            }
            current = tracker.prefix();
            current_width = 0;
        }};
    }

    for token in tokenize_with_ansi(line) {
        match token {
            Token::Ansi(code) => {
                tracker.observe(&code);
                current.push_str(&code);
            }
            Token::Space(s) => {
                let w = visible_width(&s);
                // Trailing whitespace at a break point is dropped, not carried.
                if current_width + w > width {
                    flush!();
                } else {
                    current.push_str(&s);
                    current_width += w;
                }
            }
            Token::Word(word) => {
                let w = visible_width(&word);
                if w > width {
                    // Longer than a full line: hard-break it across lines.
                    if current_width > 0 {
                        flush!();
                    }
                    let mut remaining = word.as_str();
                    while visible_width(remaining) > width {
                        let head = truncate_to_width(remaining, width, None);
                        let consumed = head.text.len();
                        current.push_str(&head.text);
                        flush!();
                        remaining = &remaining[consumed..];
                        if consumed == 0 {
                            break;
                        }
                    }
                    current.push_str(remaining);
                    current_width += visible_width(remaining);
                } else {
                    if current_width + w > width {
                        flush!();
                    }
                    current.push_str(&word);
                    current_width += w;
                }
            }
        }
    }

    if !current.is_empty() {
        let mut done = current;
        done.push_str(tracker.suffix());
        lines.push(done);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

enum Token {
    Ansi(String),
    Space(String),
    Word(String),
}

/// Split a line into escape sequences, whitespace runs, and word runs.
fn tokenize_with_ansi(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut buf = String::new();
    let mut buf_is_space = false;
    let mut i = 0;

    macro_rules! flush_buf {
        () => {
            if !buf.is_empty() {
                let taken = std::mem::take(&mut buf);
                tokens.push(if buf_is_space {
                    Token::Space(taken)
                } else {
                    Token::Word(taken)
                });
            }
        };
    }

    while i < line.len() {
        if let Some(code) = extract_ansi_code(line, i) {
            flush_buf!();
            tokens.push(Token::Ansi(code.text));
            i = code.end;
            continue;
        }
        let c = line[i..].chars().next().expect("char boundary");
        let is_space = is_whitespace_char(c);
        if !buf.is_empty() && is_space != buf_is_space {
            flush_buf!();
        }
        buf_is_space = is_space;
        buf.push(c);
        i += c.len_utf8();
    }
    flush_buf!();
    tokens
}

/// Pad `line` to exactly `width` cells, applying `bg` to the whole row.
///
/// Used by components that paint a background: the padding must be styled too,
/// otherwise the fill stops at the text and leaves a ragged edge.
pub fn apply_background_to_line(line: &str, width: usize, bg: &dyn Fn(&str) -> String) -> String {
    let visible = visible_width(line);
    let padding = width.saturating_sub(visible);
    bg(&format!("{line}{}", " ".repeat(padding)))
}

/// Pad `line` to exactly `width` cells with plain spaces.
pub fn pad_to_width(line: &str, width: usize) -> String {
    let visible = visible_width(line);
    format!("{line}{}", " ".repeat(width.saturating_sub(visible)))
}

/// Normalize terminal output for diffing: strip carriage returns and expand
/// tabs, so two lines that paint identically compare equal.
pub fn normalize_terminal_output(s: &str) -> String {
    s.replace('\r', "").replace('\t', "   ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_and_measures_sgr() {
        let s = "\u{1b}[31mred\u{1b}[0m";
        assert_eq!(strip_ansi(s), "red");
        assert_eq!(visible_width(s), 3);
    }

    #[test]
    fn measures_wide_and_emoji_clusters() {
        assert_eq!(visible_width("日本語"), 6);
        assert_eq!(visible_width("👍"), 2);
        // ZWJ family sequence is still one double-wide cell.
        assert_eq!(visible_width("\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f466}"), 2);
    }

    #[test]
    fn extracts_osc_control_string() {
        let s = "\u{1b}]8;;https://example.com\u{7}link";
        assert_eq!(strip_ansi(s), "link");
        assert_eq!(visible_width(s), 4);
    }

    #[test]
    fn truncates_without_splitting_wide_clusters() {
        let t = truncate_to_width("日本語", 3, None);
        // Only one full-width char fits in 3 cells; the second would overflow.
        assert_eq!(t.text, "日");
        assert_eq!(t.width, 2);
    }

    #[test]
    fn truncates_with_ellipsis() {
        let t = truncate_to_width("hello world", 8, Some("…"));
        assert_eq!(t.width, 8);
        assert!(t.text.ends_with('…'));
    }

    #[test]
    fn wraps_on_word_boundaries() {
        let lines = wrap_text_with_ansi("the quick brown fox", 10);
        assert_eq!(lines, vec!["the quick", "brown fox"]);
    }

    #[test]
    fn hard_breaks_overlong_words() {
        let lines = wrap_text_with_ansi("abcdefghij", 4);
        assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn wrap_reopens_active_styles() {
        let lines = wrap_text_with_ansi("\u{1b}[31maaa bbb\u{1b}[0m", 3);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].starts_with("\u{1b}[31m"));
    }

    #[test]
    fn wrap_preserves_explicit_newlines() {
        assert_eq!(wrap_text_with_ansi("a\n\nb", 10), vec!["a", "", "b"]);
    }

    #[test]
    fn slices_by_visible_column() {
        assert_eq!(slice_by_column("abcdef", 2, 3), "cde");
    }

    #[test]
    fn slice_keeps_leading_styles() {
        let out = slice_by_column("\u{1b}[31mabcdef", 2, 2);
        assert!(out.starts_with("\u{1b}[31m"));
        assert_eq!(strip_ansi(&out), "cd");
    }

    #[test]
    fn pads_ignoring_escape_bytes() {
        let padded = pad_to_width("\u{1b}[31mab\u{1b}[0m", 5);
        assert_eq!(visible_width(&padded), 5);
    }
}
