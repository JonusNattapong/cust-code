//! Differential frame renderer.
//!
//! Port of the diffing core of `pi-tui`'s `src/tui.ts`. Given the previous
//! frame's lines and the new ones, it emits the smallest ANSI byte sequence
//! that turns one into the other — repainting only the changed rows instead of
//! replaying the whole transcript, which is what keeps a streaming session from
//! flickering and scrolling from the top.
//!
//! Kept free of any terminal I/O so the algorithm can be asserted on directly.

use super::component::CURSOR_MARKER;
use super::utils::visible_width;

/// Begin synchronized output — the terminal holds the frame until the matching
/// end, so a multi-row repaint lands as one atomic update rather than a tear.
const SYNC_BEGIN: &str = "\u{1b}[?2026h";
const SYNC_END: &str = "\u{1b}[?2026l";
/// Erase from cursor to end of line.
const CLEAR_LINE: &str = "\u{1b}[2K";

/// Why a frame was painted in full rather than diffed. Surfaced for debugging
/// and asserted on in tests — a redraw reason that fires every frame is a bug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullRedrawReason {
    FirstRender,
    WidthChanged,
    HeightChanged,
    ContentShrank,
    ScrolledAboveViewport,
}

/// What a frame did, for tests and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameKind {
    /// Nothing changed; at most the hardware cursor moved.
    NoChange,
    /// Rows `[first, last]` were repainted in place.
    Partial { first: usize, last: usize },
    /// The whole content block was repainted.
    Full(FullRedrawReason),
}

/// One rendered frame: the bytes to write and what the renderer decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub output: String,
    pub kind: FrameKind,
}

/// Cursor position within the content block, in (row, column) cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPos {
    pub row: usize,
    pub col: usize,
}

/// Holds the previous frame and emits diffs against it.
#[derive(Debug, Default)]
pub struct Differ {
    previous_lines: Vec<String>,
    previous_width: usize,
    previous_height: usize,
    /// Row the terminal cursor currently sits on, relative to content top.
    cursor_row: usize,
    /// High-water mark of rows ever painted, so shrinking content can clear
    /// the rows it no longer covers.
    max_lines_rendered: usize,
    /// Repaint fully when content shrinks. Disable for append-only transcripts
    /// where the stale tail is acceptable and the redraw is not.
    clear_on_shrink: bool,
    started: bool,
}

impl Differ {
    pub fn new() -> Self {
        Self {
            clear_on_shrink: true,
            ..Default::default()
        }
    }

    pub fn set_clear_on_shrink(&mut self, enabled: bool) {
        self.clear_on_shrink = enabled;
    }

    /// Forget the previous frame so the next one repaints from scratch.
    pub fn reset(&mut self) {
        self.previous_lines.clear();
        self.previous_width = 0;
        self.previous_height = 0;
        self.cursor_row = 0;
        self.max_lines_rendered = 0;
        self.started = false;
    }

    /// The lines currently believed to be on screen.
    pub fn previous_lines(&self) -> &[String] {
        &self.previous_lines
    }

    /// Diff `lines` against the previous frame and return the bytes to write.
    ///
    /// `lines` may embed [`CURSOR_MARKER`]; it is stripped and its position
    /// becomes the hardware cursor placement for the frame.
    pub fn frame(&mut self, lines: Vec<String>, width: usize, height: usize) -> Frame {
        let (new_lines, cursor) = extract_cursor(lines);

        let width_changed = self.started && width != self.previous_width;
        let height_changed = self.started && height != self.previous_height;

        // First paint onto a screen we assume is already clean.
        if !self.started {
            return self.full_redraw(new_lines, width, height, cursor, FullRedrawReason::FirstRender, false);
        }
        // Wrapping changed under us, so no previous row maps to a new one.
        if width_changed {
            return self.full_redraw(new_lines, width, height, cursor, FullRedrawReason::WidthChanged, true);
        }
        if height_changed {
            return self.full_redraw(new_lines, width, height, cursor, FullRedrawReason::HeightChanged, true);
        }
        // Content got shorter than rows we have painted: those rows are stale.
        if self.clear_on_shrink && new_lines.len() < self.max_lines_rendered {
            return self.full_redraw(new_lines, width, height, cursor, FullRedrawReason::ContentShrank, true);
        }

        let (first_changed, last_changed) = match self.changed_span(&new_lines) {
            Some(span) => span,
            None => {
                // Identical content; the cursor may still have moved.
                let mut out = String::new();
                self.emit_cursor(&mut out, cursor, new_lines.len());
                self.previous_height = height;
                return Frame {
                    output: out,
                    kind: FrameKind::NoChange,
                };
            }
        };

        // Every change lies in rows that no longer exist: clear, don't paint.
        if first_changed >= new_lines.len() {
            return self.clear_trailing(new_lines, width, height, cursor, first_changed);
        }

        // The visible window scrolled: rows on screen no longer correspond to
        // `new_lines`, so an in-place patch would land on the wrong rows.
        let viewport_top = self.previous_lines.len().saturating_sub(height);
        if first_changed < viewport_top {
            return self.full_redraw(
                new_lines,
                width,
                height,
                cursor,
                FullRedrawReason::ScrolledAboveViewport,
                true,
            );
        }

        self.partial_redraw(new_lines, width, height, cursor, first_changed, last_changed)
    }

    /// First and last row indices that differ, accounting for appended rows.
    fn changed_span(&self, new_lines: &[String]) -> Option<(usize, usize)> {
        let mut first = None;
        let mut last = 0usize;
        let max = new_lines.len().max(self.previous_lines.len());
        for i in 0..max {
            let old = self.previous_lines.get(i).map(String::as_str).unwrap_or("");
            let new = new_lines.get(i).map(String::as_str).unwrap_or("");
            if old != new {
                first.get_or_insert(i);
                last = i;
            }
        }
        let mut first = first?;
        if new_lines.len() > self.previous_lines.len() {
            first = first.min(self.previous_lines.len());
            last = new_lines.len() - 1;
        }
        Some((first, last))
    }

    fn full_redraw(
        &mut self,
        new_lines: Vec<String>,
        width: usize,
        height: usize,
        cursor: Option<CursorPos>,
        reason: FullRedrawReason,
        clear: bool,
    ) -> Frame {
        let mut out = String::from(SYNC_BEGIN);

        if clear {
            // Walk back to the top of the block we own, then wipe downward.
            if self.cursor_row > 0 {
                out.push_str(&format!("\u{1b}[{}A", self.cursor_row));
            }
            out.push('\r');
            // Erase from the cursor to the end of the screen.
            out.push_str("\u{1b}[0J");
        }

        // On a clear+repaint we can only own the visible window; anything above
        // it belongs to scrollback and must not be replayed.
        let start = if clear && new_lines.len() > height {
            new_lines.len() - height
        } else {
            0
        };

        for (i, line) in new_lines[start..].iter().enumerate() {
            if i > 0 {
                out.push_str("\r\n");
            }
            out.push_str(CLEAR_LINE);
            out.push('\r');
            out.push_str(line);
        }

        self.cursor_row = new_lines.len().saturating_sub(1);
        out.push_str(SYNC_END);

        self.commit(new_lines, width, height);
        self.emit_cursor(&mut out, cursor, self.previous_lines.len());
        Frame {
            output: out,
            kind: FrameKind::Full(reason),
        }
    }

    fn partial_redraw(
        &mut self,
        new_lines: Vec<String>,
        width: usize,
        height: usize,
        cursor: Option<CursorPos>,
        first: usize,
        last: usize,
    ) -> Frame {
        let mut out = String::from(SYNC_BEGIN);
        out.push_str(&self.move_to_row(first));

        for (offset, line) in new_lines[first..=last.min(new_lines.len() - 1)].iter().enumerate() {
            if offset > 0 {
                out.push_str("\r\n");
            }
            out.push_str(CLEAR_LINE);
            out.push('\r');
            out.push_str(line);
        }
        self.cursor_row = last.min(new_lines.len() - 1);

        // Rows the previous frame painted below the new content are now stale.
        let extra = self.previous_lines.len().saturating_sub(new_lines.len());
        for _ in 0..extra {
            out.push_str("\r\n");
            out.push_str(CLEAR_LINE);
            self.cursor_row += 1;
        }

        out.push_str(SYNC_END);
        self.commit(new_lines, width, height);
        self.emit_cursor(&mut out, cursor, self.previous_lines.len());
        Frame {
            output: out,
            kind: FrameKind::Partial { first, last },
        }
    }

    fn clear_trailing(
        &mut self,
        new_lines: Vec<String>,
        width: usize,
        height: usize,
        cursor: Option<CursorPos>,
        first: usize,
    ) -> Frame {
        let mut out = String::from(SYNC_BEGIN);
        let target = new_lines.len().saturating_sub(1);
        out.push_str(&self.move_to_row(target));

        let extra = self.previous_lines.len().saturating_sub(new_lines.len());
        for _ in 0..extra {
            out.push_str("\r\n");
            out.push_str(CLEAR_LINE);
            self.cursor_row += 1;
        }
        // Return to the last real content row so the next frame's arithmetic
        // starts from the block, not from the cleared tail.
        if extra > 0 {
            out.push_str(&format!("\u{1b}[{extra}A"));
            self.cursor_row -= extra;
        }
        out.push_str(SYNC_END);

        self.commit(new_lines, width, height);
        self.emit_cursor(&mut out, cursor, self.previous_lines.len());
        Frame {
            output: out,
            kind: FrameKind::Partial { first, last: first },
        }
    }

    /// Vertical move from the tracked cursor row to `row`, plus a carriage
    /// return so the column is known.
    fn move_to_row(&mut self, row: usize) -> String {
        let mut out = String::new();
        if row > self.cursor_row {
            out.push_str(&format!("\u{1b}[{}B", row - self.cursor_row));
        } else if row < self.cursor_row {
            out.push_str(&format!("\u{1b}[{}A", self.cursor_row - row));
        }
        out.push('\r');
        self.cursor_row = row;
        out
    }

    /// Park the hardware cursor at `cursor`, or leave it after the content.
    fn emit_cursor(&mut self, out: &mut String, cursor: Option<CursorPos>, total_lines: usize) {
        let Some(pos) = cursor else { return };
        let row = pos.row.min(total_lines.saturating_sub(1));
        out.push_str(&self.move_to_row(row));
        if pos.col > 0 {
            out.push_str(&format!("\u{1b}[{}C", pos.col));
        }
    }

    fn commit(&mut self, new_lines: Vec<String>, width: usize, height: usize) {
        self.max_lines_rendered = self.max_lines_rendered.max(new_lines.len());
        self.previous_lines = new_lines;
        self.previous_width = width;
        self.previous_height = height;
        self.started = true;
    }
}

/// Strip [`CURSOR_MARKER`] from the lines, returning where it was.
fn extract_cursor(lines: Vec<String>) -> (Vec<String>, Option<CursorPos>) {
    let mut cursor = None;
    let out = lines
        .into_iter()
        .enumerate()
        .map(|(row, line)| match line.find(CURSOR_MARKER) {
            Some(idx) if cursor.is_none() => {
                let col = visible_width(&line[..idx]);
                cursor = Some(CursorPos { row, col });
                line.replace(CURSOR_MARKER, "")
            }
            // A second marker is a component bug; drop it rather than fight
            // over the cursor.
            Some(_) => line.replace(CURSOR_MARKER, ""),
            None => line,
        })
        .collect();
    (out, cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn first_frame_is_a_full_paint_without_clearing() {
        let mut d = Differ::new();
        let f = d.frame(lines(&["a", "b"]), 80, 24);
        assert_eq!(f.kind, FrameKind::Full(FullRedrawReason::FirstRender));
        // No erase-to-end-of-screen: the screen is assumed clean.
        assert!(!f.output.contains("\u{1b}[0J"));
        assert!(f.output.contains('a') && f.output.contains('b'));
    }

    #[test]
    fn identical_content_emits_nothing() {
        let mut d = Differ::new();
        d.frame(lines(&["a", "b"]), 80, 24);
        let f = d.frame(lines(&["a", "b"]), 80, 24);
        assert_eq!(f.kind, FrameKind::NoChange);
        assert_eq!(f.output, "");
    }

    #[test]
    fn single_changed_row_repaints_only_that_row() {
        let mut d = Differ::new();
        d.frame(lines(&["a", "b", "c"]), 80, 24);
        let f = d.frame(lines(&["a", "X", "c"]), 80, 24);
        assert_eq!(f.kind, FrameKind::Partial { first: 1, last: 1 });
        assert!(f.output.contains('X'));
        // Untouched rows are not re-sent.
        assert!(!f.output.contains('c'));
    }

    #[test]
    fn appended_rows_paint_from_the_old_end() {
        let mut d = Differ::new();
        d.frame(lines(&["a"]), 80, 24);
        let f = d.frame(lines(&["a", "b", "c"]), 80, 24);
        assert_eq!(f.kind, FrameKind::Partial { first: 1, last: 2 });
        assert!(f.output.contains('b') && f.output.contains('c'));
        assert!(!f.output.contains("\u{1b}[0J"));
    }

    #[test]
    fn width_change_forces_a_full_repaint() {
        let mut d = Differ::new();
        d.frame(lines(&["a"]), 80, 24);
        let f = d.frame(lines(&["a"]), 100, 24);
        assert_eq!(f.kind, FrameKind::Full(FullRedrawReason::WidthChanged));
        assert!(f.output.contains("\u{1b}[0J"));
    }

    #[test]
    fn height_change_forces_a_full_repaint() {
        let mut d = Differ::new();
        d.frame(lines(&["a"]), 80, 24);
        let f = d.frame(lines(&["a"]), 80, 30);
        assert_eq!(f.kind, FrameKind::Full(FullRedrawReason::HeightChanged));
    }

    #[test]
    fn shrinking_content_repaints_fully_by_default() {
        let mut d = Differ::new();
        d.frame(lines(&["a", "b", "c"]), 80, 24);
        let f = d.frame(lines(&["a"]), 80, 24);
        assert_eq!(f.kind, FrameKind::Full(FullRedrawReason::ContentShrank));
    }

    #[test]
    fn shrinking_clears_trailing_rows_when_full_repaint_is_off() {
        let mut d = Differ::new();
        d.set_clear_on_shrink(false);
        d.frame(lines(&["a", "b", "c"]), 80, 24);
        let f = d.frame(lines(&["a", "b"]), 80, 24);
        // Row 2 vanished, rows 0-1 are unchanged: nothing to paint, one to wipe.
        assert!(f.output.contains(CLEAR_LINE));
        assert!(!f.output.contains('a'));
    }

    #[test]
    fn full_repaint_only_replays_the_visible_window() {
        let mut d = Differ::new();
        let tall: Vec<String> = (0..10).map(|i| format!("row{i}")).collect();
        d.frame(tall.clone(), 80, 4);
        // Width change forces a clear+repaint; scrollback must not be replayed.
        let f = d.frame(tall, 90, 4);
        assert!(!f.output.contains("row0"));
        assert!(f.output.contains("row9"));
    }

    #[test]
    fn cursor_marker_is_stripped_and_positions_the_cursor() {
        let mut d = Differ::new();
        let f = d.frame(vec![format!("ab{CURSOR_MARKER}cd")], 80, 24);
        assert!(!f.output.contains(CURSOR_MARKER));
        assert!(f.output.contains("abcd"));
        // Cursor parked at visible column 2.
        assert!(f.output.contains("\u{1b}[2C"));
    }

    #[test]
    fn cursor_column_accounts_for_escape_sequences() {
        let (out, cursor) = extract_cursor(vec![format!("\u{1b}[31mab\u{1b}[0m{CURSOR_MARKER}c")]);
        assert_eq!(cursor, Some(CursorPos { row: 0, col: 2 }));
        assert!(!out[0].contains(CURSOR_MARKER));
    }

    #[test]
    fn cursor_move_alone_emits_no_content() {
        let mut d = Differ::new();
        d.frame(vec![format!("{CURSOR_MARKER}ab"), "cd".into()], 80, 24);
        let f = d.frame(vec!["ab".into(), format!("c{CURSOR_MARKER}d")], 80, 24);
        assert_eq!(f.kind, FrameKind::NoChange);
        assert!(!f.output.contains("ab"));
        assert!(f.output.contains("\u{1b}[1C"));
    }

    #[test]
    fn frames_are_wrapped_in_synchronized_output() {
        let mut d = Differ::new();
        let f = d.frame(lines(&["a"]), 80, 24);
        assert!(f.output.starts_with(SYNC_BEGIN));
        assert!(f.output.contains(SYNC_END));
    }

    #[test]
    fn reset_makes_the_next_frame_a_first_render() {
        let mut d = Differ::new();
        d.frame(lines(&["a"]), 80, 24);
        d.reset();
        let f = d.frame(lines(&["a"]), 80, 24);
        assert_eq!(f.kind, FrameKind::Full(FullRedrawReason::FirstRender));
    }
}
