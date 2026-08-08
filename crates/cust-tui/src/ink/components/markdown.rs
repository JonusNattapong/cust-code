//! Markdown-to-terminal rendering.
//!
//! A reduced port of pi-tui's `components/markdown.ts` (~1k lines): headings,
//! paragraphs, fenced code blocks, bulleted/numbered lists, blockquotes, and
//! inline bold/italic/code spans. Assistant replies are markdown; this is
//! what turns them into styled terminal rows.
//!
//! Deferred from the full port: tables, nested lists, link rendering as OSC 8
//! hyperlinks, and syntax highlighting inside fenced code (fences render
//! as plain monospace text, dimmed).

use crate::ink::utils::{visible_width, wrap_text_with_ansi};
use crate::ink::Component;

const DIM: &str = "\u{1b}[2m";
const BOLD: &str = "\u{1b}[1m";
const ITALIC: &str = "\u{1b}[3m";
const CYAN: &str = "\u{1b}[36m";
const RESET: &str = "\u{1b}[0m";

/// Parsed block-level markdown elements.
#[derive(Debug, Clone, PartialEq)]
enum Block {
    Heading { level: u8, text: String },
    Paragraph(String),
    CodeFence { lang: Option<String>, lines: Vec<String> },
    BulletItem { text: String, indent: usize },
    NumberedItem { number: u32, text: String, indent: usize },
    Blockquote(String),
    Blank,
}

/// Split raw markdown into block-level elements, line by line.
///
/// Deliberately line-oriented rather than a real parser: paragraphs are runs
/// of non-blank, non-special lines joined by spaces, which matches how
/// streamed LLM output actually arrives (one flush per line).
fn parse_blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines = text.lines().peekable();
    let mut paragraph_buf: Vec<String> = Vec::new();

    macro_rules! flush_paragraph {
        () => {
            if !paragraph_buf.is_empty() {
                blocks.push(Block::Paragraph(paragraph_buf.join(" ")));
                paragraph_buf.clear();
            }
        };
    }

    while let Some(line) = lines.next() {
        let trimmed = line.trim_end();

        if let Some(fence_lang) = trimmed.trim_start().strip_prefix("```") {
            flush_paragraph!();
            let lang = (!fence_lang.is_empty()).then(|| fence_lang.to_string());
            let mut code_lines = Vec::new();
            for inner in lines.by_ref() {
                if inner.trim_start().starts_with("```") {
                    break;
                }
                code_lines.push(inner.to_string());
            }
            blocks.push(Block::CodeFence {
                lang,
                lines: code_lines,
            });
            continue;
        }

        if trimmed.trim().is_empty() {
            flush_paragraph!();
            blocks.push(Block::Blank);
            continue;
        }

        let stripped = trimmed.trim_start();
        let indent = trimmed.len() - stripped.len();

        if let Some(rest) = stripped.strip_prefix('#') {
            let mut level = 1u8;
            let mut r = rest;
            while let Some(more) = r.strip_prefix('#') {
                level += 1;
                r = more;
            }
            if r.starts_with(' ') || r.is_empty() {
                flush_paragraph!();
                blocks.push(Block::Heading {
                    level: level.min(6),
                    text: r.trim().to_string(),
                });
                continue;
            }
        }

        if let Some(rest) = stripped.strip_prefix("> ") {
            flush_paragraph!();
            blocks.push(Block::Blockquote(rest.to_string()));
            continue;
        }
        if stripped == ">" {
            flush_paragraph!();
            blocks.push(Block::Blockquote(String::new()));
            continue;
        }

        if let Some(rest) = stripped.strip_prefix("- ").or_else(|| stripped.strip_prefix("* ")) {
            flush_paragraph!();
            blocks.push(Block::BulletItem {
                text: rest.to_string(),
                indent,
            });
            continue;
        }

        if let Some((num, rest)) = parse_numbered_item(stripped) {
            flush_paragraph!();
            blocks.push(Block::NumberedItem {
                number: num,
                text: rest,
                indent,
            });
            continue;
        }

        paragraph_buf.push(trimmed.to_string());
    }
    flush_paragraph!();
    blocks
}

/// Parse a `"N. rest"` numbered-list marker.
fn parse_numbered_item(line: &str) -> Option<(u32, String)> {
    let dot = line.find(". ")?;
    let digits = &line[..dot];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let num: u32 = digits.parse().ok()?;
    Some((num, line[dot + 2..].to_string()))
}

/// Apply inline `**bold**`, `*italic*`, and `` `code` `` spans as ANSI codes.
///
/// A single left-to-right scan: whichever marker appears first is honoured,
/// so `**bold with *stray* star**` doesn't get its outer span cut short by
/// the inner one.
fn render_inline(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if text[i..].starts_with("**") {
            if let Some(end) = text[i + 2..].find("**") {
                out.push_str(BOLD);
                out.push_str(&render_inline(&text[i + 2..i + 2 + end]));
                out.push_str(RESET);
                i += 2 + end + 2;
                continue;
            }
        } else if text[i..].starts_with('`') {
            if let Some(end) = text[i + 1..].find('`') {
                out.push_str(CYAN);
                out.push_str(&text[i + 1..i + 1 + end]);
                out.push_str(RESET);
                i += 1 + end + 1;
                continue;
            }
        } else if text[i..].starts_with('*') && !text[i..].starts_with("**") {
            if let Some(end) = text[i + 1..].find('*') {
                out.push_str(ITALIC);
                out.push_str(&render_inline(&text[i + 1..i + 1 + end]));
                out.push_str(RESET);
                i += 1 + end + 1;
                continue;
            }
        }
        let ch = text[i..].chars().next().expect("valid char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Renders markdown text to wrapped, styled terminal lines.
pub struct Markdown {
    text: String,
    cache: Option<(String, usize, Vec<String>)>,
}

impl Markdown {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            cache: None,
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cache = None;
    }

    fn render_blocks(&self, width: usize) -> Vec<String> {
        let mut out = Vec::new();
        for block in parse_blocks(&self.text) {
            match block {
                Block::Blank => out.push(String::new()),
                Block::Heading { level, text } => {
                    let prefix = "#".repeat(level as usize);
                    let styled = format!("{BOLD}{CYAN}{prefix} {}{RESET}", render_inline(&text));
                    out.extend(wrap_text_with_ansi(&styled, width));
                }
                Block::Paragraph(text) => {
                    out.extend(wrap_text_with_ansi(&render_inline(&text), width));
                }
                Block::CodeFence { lang, lines } => {
                    if let Some(lang) = &lang {
                        out.push(format!("{DIM}```{lang}{RESET}"));
                    } else {
                        out.push(format!("{DIM}```{RESET}"));
                    }
                    for line in lines {
                        out.push(format!("{DIM}{line}{RESET}"));
                    }
                    out.push(format!("{DIM}```{RESET}"));
                }
                Block::BulletItem { text, indent } => {
                    let pad = " ".repeat(indent);
                    let marker = format!("{pad}{CYAN}\u{2022}{RESET} ");
                    out.extend(wrap_continuation(&marker, &render_inline(&text), width));
                }
                Block::NumberedItem { number, text, indent } => {
                    let pad = " ".repeat(indent);
                    let marker = format!("{pad}{CYAN}{number}.{RESET} ");
                    out.extend(wrap_continuation(&marker, &render_inline(&text), width));
                }
                Block::Blockquote(text) => {
                    let marker = format!("{DIM}\u{2502} {RESET}");
                    out.extend(wrap_continuation(&marker, &render_inline(&text), width));
                }
            }
        }
        out
    }
}

/// Wrap `text` at `width`, prefixing the first line with `marker` and
/// indenting continuation lines to align under it.
fn wrap_continuation(marker: &str, text: &str, width: usize) -> Vec<String> {
    let marker_width = visible_width(marker);
    let content_width = width.saturating_sub(marker_width).max(1);
    let wrapped = wrap_text_with_ansi(text, content_width);
    let indent = " ".repeat(marker_width);
    wrapped
        .into_iter()
        .enumerate()
        .map(|(i, line)| if i == 0 { format!("{marker}{line}") } else { format!("{indent}{line}") })
        .collect()
}

impl Component for Markdown {
    fn render(&mut self, width: usize) -> Vec<String> {
        if let Some((cached_text, cached_width, lines)) = &self.cache {
            if cached_text == &self.text && *cached_width == width {
                return lines.clone();
            }
        }
        let lines = self.render_blocks(width);
        self.cache = Some((self.text.clone(), width, lines.clone()));
        lines
    }

    fn invalidate(&mut self) {
        self.cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::utils::strip_ansi;

    fn plain(md: &str, width: usize) -> Vec<String> {
        Markdown::new(md).render_blocks(width).iter().map(|l| strip_ansi(l)).collect()
    }

    #[test]
    fn renders_a_heading() {
        let out = plain("# Title", 40);
        assert_eq!(out, vec!["# Title"]);
    }

    #[test]
    fn renders_a_wrapped_paragraph() {
        let out = plain("the quick brown fox jumps", 10);
        assert_eq!(out, vec!["the quick", "brown fox", "jumps"]);
    }

    #[test]
    fn blank_line_separates_blocks() {
        let out = plain("a\n\nb", 20);
        assert_eq!(out, vec!["a", "", "b"]);
    }

    #[test]
    fn renders_a_fenced_code_block_verbatim() {
        let out = plain("```rust\nfn main() {}\n```", 40);
        assert_eq!(out, vec!["```rust", "fn main() {}", "```"]);
    }

    #[test]
    fn code_fence_is_not_word_wrapped() {
        let long_line = "x".repeat(50);
        let md = format!("```\n{long_line}\n```");
        let out = plain(&md, 10);
        // Code lines pass through untouched even past the render width.
        assert!(out.iter().any(|l| l.len() == 50));
    }

    #[test]
    fn renders_bullet_list_with_marker() {
        let out = plain("- one\n- two", 40);
        assert_eq!(out, vec!["\u{2022} one", "\u{2022} two"]);
    }

    #[test]
    fn renders_numbered_list_with_its_number() {
        let out = plain("1. first\n2. second", 40);
        assert_eq!(out, vec!["1. first", "2. second"]);
    }

    #[test]
    fn wrapped_list_item_indents_continuation() {
        let out = plain("- a long item that needs to wrap across lines", 15);
        assert!(out.len() > 1);
        assert!(out[0].starts_with('\u{2022}'));
        assert!(out[1].starts_with("  ")); // aligned under the marker+space
    }

    #[test]
    fn renders_blockquote_with_bar() {
        let out = plain("> quoted text", 40);
        assert_eq!(out, vec!["\u{2502} quoted text"]);
    }

    #[test]
    fn inline_bold_strips_to_plain_text() {
        let out = plain("this is **bold** text", 40);
        assert_eq!(out, vec!["this is bold text"]);
    }

    #[test]
    fn inline_bold_applies_ansi() {
        let mut m = Markdown::new("**bold**");
        let out = m.render(40);
        assert!(out[0].contains(BOLD));
        assert!(out[0].contains(RESET));
    }

    #[test]
    fn inline_code_span_applies_ansi_and_preserves_content() {
        let mut m = Markdown::new("run `cargo test` now");
        let out = m.render(40);
        assert!(strip_ansi(&out[0]) == "run cargo test now");
        assert!(out[0].contains(CYAN));
    }

    #[test]
    fn inline_italic_does_not_confuse_bold_marker() {
        let out = plain("**bold with *stray* star**", 60);
        assert_eq!(out, vec!["bold with stray star"]);
    }

    #[test]
    fn unterminated_inline_marker_is_left_literal() {
        let out = plain("a * lone star", 40);
        assert_eq!(out, vec!["a * lone star"]);
    }

    #[test]
    fn cache_hits_until_text_or_width_changes() {
        let mut m = Markdown::new("hello there world");
        let a = m.render(40);
        assert_eq!(a, m.render(40));
        assert_ne!(a, m.render(5));
        m.set_text("bye");
        assert_ne!(a, m.render(40));
    }
}
