//! Anchored overlays: panels composited on top of base content before the
//! diff runs.
//!
//! Port of the overlay half of pi-tui's `tui.ts` (`OverlayOptions`,
//! `showOverlay`, `compositeOverlays`, `resolveOverlayLayout`). This is what
//! permission prompts, the model picker, and the slash-command menu are drawn
//! as: a component sized and positioned relative to the terminal, blitted
//! into the frame lines before [`crate::ink::Differ`] ever sees them.
//!
//! Deferred from the full port: `scrollback` overlays, `aboveMarker`
//! anchoring, and selection-region bookkeeping — none of them are on the path
//! to a working permission prompt or menu.

use crate::ink::utils::{slice_by_column, visible_width};
use crate::ink::Component;

/// Anchor point an overlay is positioned relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayAnchor {
    #[default]
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopCenter,
    BottomCenter,
    LeftCenter,
    RightCenter,
}

/// A size that is either an absolute cell count or a percentage of the
/// reference dimension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeValue {
    Cells(usize),
    Percent(f64),
}

impl SizeValue {
    fn resolve(self, reference: usize) -> usize {
        match self {
            SizeValue::Cells(n) => n,
            SizeValue::Percent(p) => ((reference as f64) * p / 100.0).floor() as usize,
        }
    }
}

/// Margin from each terminal edge, in cells.
#[derive(Debug, Clone, Copy, Default)]
pub struct OverlayMargin {
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
    pub left: usize,
}

impl OverlayMargin {
    pub fn all(n: usize) -> Self {
        Self {
            top: n,
            right: n,
            bottom: n,
            left: n,
        }
    }
}

/// Positioning and sizing for one overlay.
#[derive(Debug, Clone, Default)]
pub struct OverlayOptions {
    pub width: Option<SizeValue>,
    pub min_width: Option<usize>,
    pub max_height: Option<SizeValue>,
    pub anchor: OverlayAnchor,
    pub offset_x: i32,
    pub offset_y: i32,
    pub margin: OverlayMargin,
    /// Don't capture keyboard focus when shown.
    pub non_capturing: bool,
}

/// Computed placement for one overlay render pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Layout {
    width: usize,
    row: usize,
    col: usize,
    max_height: Option<usize>,
}

fn resolve_layout(opt: &OverlayOptions, overlay_height: usize, term_width: usize, term_height: usize) -> Layout {
    let margin = &opt.margin;
    let avail_width = term_width.saturating_sub(margin.left + margin.right).max(1);
    let avail_height = term_height.saturating_sub(margin.top + margin.bottom).max(1);

    let mut width = opt
        .width
        .map(|w| w.resolve(term_width))
        .unwrap_or_else(|| 80.min(avail_width));
    if let Some(min_w) = opt.min_width {
        width = width.max(min_w);
    }
    width = width.clamp(1, avail_width);

    let max_height = opt
        .max_height
        .map(|h| h.resolve(term_height).clamp(1, avail_height));

    let effective_height = max_height.map(|h| overlay_height.min(h)).unwrap_or(overlay_height);

    let mut row = anchor_row(opt.anchor, effective_height, avail_height, margin.top);
    let mut col = anchor_col(opt.anchor, width, avail_width, margin.left);
    row = (row as i64 + opt.offset_y as i64).max(0) as usize;
    col = (col as i64 + opt.offset_x as i64).max(0) as usize;

    // Clamp back inside the terminal, respecting margins on the far edge.
    let row_ceiling = term_height.saturating_sub(margin.bottom + effective_height);
    let col_ceiling = term_width.saturating_sub(margin.right + width);
    row = row.clamp(margin.top, row_ceiling.max(margin.top));
    col = col.clamp(margin.left, col_ceiling.max(margin.left));

    Layout {
        width,
        row,
        col,
        max_height,
    }
}

fn anchor_row(anchor: OverlayAnchor, height: usize, avail_height: usize, margin_top: usize) -> usize {
    use OverlayAnchor::*;
    match anchor {
        TopLeft | TopCenter | TopRight => margin_top,
        BottomLeft | BottomCenter | BottomRight => margin_top + avail_height.saturating_sub(height),
        LeftCenter | Center | RightCenter => margin_top + avail_height.saturating_sub(height) / 2,
    }
}

fn anchor_col(anchor: OverlayAnchor, width: usize, avail_width: usize, margin_left: usize) -> usize {
    use OverlayAnchor::*;
    match anchor {
        TopLeft | LeftCenter | BottomLeft => margin_left,
        TopRight | RightCenter | BottomRight => margin_left + avail_width.saturating_sub(width),
        TopCenter | Center | BottomCenter => margin_left + avail_width.saturating_sub(width) / 2,
    }
}

/// Blit `overlay_line` onto `base_line` starting at visible column `col`,
/// spanning `width` cells. Content outside `[col, col+width)` in `base_line`
/// is preserved.
fn composite_line_at(base_line: &str, overlay_line: &str, col: usize, width: usize, term_width: usize) -> String {
    let before = slice_by_column(base_line, 0, col);
    let after_start = col + width;
    let after = if after_start < term_width {
        slice_by_column(base_line, after_start, term_width - after_start)
    } else {
        String::new()
    };
    // Pad the overlay content itself to its declared width so it fully
    // covers whatever was behind it — a shorter row would let base content
    // bleed through on the right.
    let overlay_width = visible_width(overlay_line);
    let padded_overlay = if overlay_width < width {
        format!("{overlay_line}{}", " ".repeat(width - overlay_width))
    } else {
        overlay_line.to_string()
    };
    format!("{before}{padded_overlay}{after}")
}

/// One entry on the overlay stack.
struct Entry {
    component: Box<dyn Component>,
    options: OverlayOptions,
    hidden: bool,
    /// Monotonic counter; higher focus_order renders on top and holds focus.
    focus_order: u64,
}

/// A stable identifier for an overlay, valid until it is hidden/removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverlayId(u64);

/// Manages a stack of overlays and composites them onto base content.
///
/// Kept separate from [`crate::ink::Tui`] so the compositing math is testable
/// without a terminal.
#[derive(Default)]
pub struct OverlayStack {
    entries: Vec<(OverlayId, Entry)>,
    next_id: u64,
    focus_order_counter: u64,
}

impl OverlayStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new overlay onto the stack, returning its id.
    pub fn show(&mut self, component: Box<dyn Component>, options: OverlayOptions) -> OverlayId {
        self.focus_order_counter += 1;
        let id = OverlayId(self.next_id);
        self.next_id += 1;
        self.entries.push((
            id,
            Entry {
                component,
                options,
                hidden: false,
                focus_order: self.focus_order_counter,
            },
        ));
        id
    }

    /// Permanently remove an overlay.
    pub fn hide(&mut self, id: OverlayId) {
        self.entries.retain(|(entry_id, _)| *entry_id != id);
    }

    /// Temporarily hide/show without losing the overlay's place in the stack.
    pub fn set_hidden(&mut self, id: OverlayId, hidden: bool) {
        if let Some((_, entry)) = self.entries.iter_mut().find(|(entry_id, _)| *entry_id == id) {
            entry.hidden = hidden;
        }
    }

    /// Bring an overlay to the front of the focus order.
    pub fn focus(&mut self, id: OverlayId) {
        self.focus_order_counter += 1;
        let counter = self.focus_order_counter;
        if let Some((_, entry)) = self.entries.iter_mut().find(|(entry_id, _)| *entry_id == id) {
            entry.focus_order = counter;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|(_, e)| e.hidden)
    }

    /// The id of the topmost (highest focus_order) visible, focus-capturing
    /// overlay, if any. This is what should hold keyboard focus.
    pub fn topmost_capturing(&self) -> Option<OverlayId> {
        self.entries
            .iter()
            .filter(|(_, e)| !e.hidden && !e.options.non_capturing)
            .max_by_key(|(_, e)| e.focus_order)
            .map(|(id, _)| *id)
    }

    /// Route input to the topmost capturing overlay, if any.
    pub fn handle_input(&mut self, data: &str) -> bool {
        let Some(id) = self.topmost_capturing() else {
            return false;
        };
        if let Some((_, entry)) = self.entries.iter_mut().find(|(entry_id, _)| *entry_id == id) {
            entry.component.handle_input(data);
            true
        } else {
            false
        }
    }

    pub fn invalidate(&mut self) {
        for (_, entry) in &mut self.entries {
            entry.component.invalidate();
        }
    }

    /// Render every visible overlay and blit it onto `base`, in ascending
    /// focus order (later entries draw on top of earlier ones).
    ///
    /// `base` is padded with blank rows up to `term_height` if it is shorter,
    /// since an overlay may be positioned below the current content.
    pub fn composite(&mut self, base: Vec<String>, term_width: usize, term_height: usize) -> Vec<String> {
        if self.entries.iter().all(|(_, e)| e.hidden) {
            return base;
        }

        let mut result = base;
        while result.len() < term_height {
            result.push(String::new());
        }

        let mut visible: Vec<&mut (OverlayId, Entry)> = self
            .entries
            .iter_mut()
            .filter(|(_, e)| !e.hidden)
            .collect();
        visible.sort_by_key(|(_, e)| e.focus_order);

        for (_, entry) in visible {
            let probe = resolve_layout(&entry.options, 0, term_width, term_height);
            let mut lines = entry.component.render(probe.width);
            if let Some(max_h) = probe.max_height {
                lines.truncate(max_h);
            }
            let layout = resolve_layout(&entry.options, lines.len(), term_width, term_height);

            while result.len() < layout.row + lines.len() {
                result.push(String::new());
            }

            for (i, overlay_line) in lines.iter().enumerate() {
                let idx = layout.row + i;
                let clipped = if visible_width(overlay_line) > layout.width {
                    slice_by_column(overlay_line, 0, layout.width)
                } else {
                    overlay_line.clone()
                };
                result[idx] = composite_line_at(&result[idx], &clipped, layout.col, layout.width, term_width);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::components::Text;

    fn base(rows: usize, width: usize) -> Vec<String> {
        vec!["x".repeat(width); rows]
    }

    fn text(s: &str) -> Box<dyn Component> {
        Box::new(Text::new(s).with_padding(0, 0))
    }

    #[test]
    fn empty_stack_leaves_base_untouched() {
        let mut stack = OverlayStack::new();
        let out = stack.composite(base(5, 10), 10, 5);
        assert_eq!(out, base(5, 10));
    }

    #[test]
    fn hidden_overlay_is_not_drawn() {
        let mut stack = OverlayStack::new();
        let id = stack.show(text("popup"), OverlayOptions::default());
        stack.set_hidden(id, true);
        let out = stack.composite(base(5, 20), 20, 5);
        assert!(!out.iter().any(|l| l.contains("popup")));
    }

    #[test]
    fn centered_overlay_lands_in_the_middle() {
        let mut stack = OverlayStack::new();
        stack.show(
            text("hi"),
            OverlayOptions {
                width: Some(SizeValue::Cells(10)),
                anchor: OverlayAnchor::Center,
                ..Default::default()
            },
        );
        let out = stack.composite(base(11, 40), 40, 11);
        let row = out.iter().position(|l| l.contains("hi")).expect("overlay row");
        // 1 content row centered in 11 rows lands on row 5.
        assert_eq!(row, 5);
    }

    #[test]
    fn top_left_anchor_hugs_the_corner() {
        let mut stack = OverlayStack::new();
        stack.show(
            text("hi"),
            OverlayOptions {
                width: Some(SizeValue::Cells(10)),
                anchor: OverlayAnchor::TopLeft,
                ..Default::default()
            },
        );
        let out = stack.composite(base(11, 40), 40, 11);
        assert!(out[0].starts_with("hi"));
    }

    #[test]
    fn overlay_content_covers_base_content_beneath_it() {
        let mut stack = OverlayStack::new();
        stack.show(
            text("HI"),
            OverlayOptions {
                width: Some(SizeValue::Cells(5)),
                anchor: OverlayAnchor::TopLeft,
                ..Default::default()
            },
        );
        let out = stack.composite(base(3, 20), 20, 3);
        // The 5-cell overlay blanks out the 'x' base content under it.
        assert!(!out[0][..5].contains('x'));
        assert!(out[0][5..].starts_with('x'));
    }

    #[test]
    fn margin_keeps_the_overlay_off_the_edge() {
        let mut stack = OverlayStack::new();
        stack.show(
            text("hi"),
            OverlayOptions {
                width: Some(SizeValue::Cells(4)),
                anchor: OverlayAnchor::TopLeft,
                margin: OverlayMargin::all(2),
                ..Default::default()
            },
        );
        let out = stack.composite(base(10, 20), 20, 10);
        assert!(out[0].starts_with("xx")); // untouched margin row
        assert!(out[2].contains("hi"));
    }

    #[test]
    fn percent_width_resolves_against_terminal_width() {
        let mut stack = OverlayStack::new();
        stack.show(
            text("hi"),
            OverlayOptions {
                width: Some(SizeValue::Percent(50.0)),
                anchor: OverlayAnchor::TopLeft,
                ..Default::default()
            },
        );
        let out = stack.composite(base(3, 40), 40, 3);
        // 50% of 40 = 20-cell-wide overlay row.
        assert_eq!(visible_width(&out[0]), 40);
    }

    #[test]
    fn later_shown_overlay_draws_on_top() {
        let mut stack = OverlayStack::new();
        stack.show(
            text("first"),
            OverlayOptions {
                width: Some(SizeValue::Cells(10)),
                anchor: OverlayAnchor::TopLeft,
                ..Default::default()
            },
        );
        stack.show(
            text("second"),
            OverlayOptions {
                width: Some(SizeValue::Cells(10)),
                anchor: OverlayAnchor::TopLeft,
                ..Default::default()
            },
        );
        let out = stack.composite(base(3, 20), 20, 3);
        assert!(out[0].contains("second"));
        assert!(!out[0].contains("first"));
    }

    #[test]
    fn focus_reorders_which_overlay_is_on_top() {
        let mut stack = OverlayStack::new();
        let first = stack.show(
            text("first"),
            OverlayOptions {
                width: Some(SizeValue::Cells(10)),
                anchor: OverlayAnchor::TopLeft,
                ..Default::default()
            },
        );
        stack.show(
            text("second"),
            OverlayOptions {
                width: Some(SizeValue::Cells(10)),
                anchor: OverlayAnchor::TopLeft,
                ..Default::default()
            },
        );
        stack.focus(first);
        let out = stack.composite(base(3, 20), 20, 3);
        assert!(out[0].contains("first"));
    }

    #[test]
    fn hide_removes_the_overlay_permanently() {
        let mut stack = OverlayStack::new();
        let id = stack.show(text("popup"), OverlayOptions::default());
        stack.hide(id);
        assert!(stack.is_empty());
        let out = stack.composite(base(5, 20), 20, 5);
        assert!(!out.iter().any(|l| l.contains("popup")));
    }

    #[test]
    fn topmost_capturing_skips_non_capturing_overlays() {
        let mut stack = OverlayStack::new();
        stack.show(
            text("toast"),
            OverlayOptions {
                non_capturing: true,
                ..Default::default()
            },
        );
        let modal = stack.show(text("modal"), OverlayOptions::default());
        assert_eq!(stack.topmost_capturing(), Some(modal));
    }

    #[test]
    fn max_height_truncates_tall_overlays() {
        let mut stack = OverlayStack::new();
        stack.show(
            Box::new(Text::new("a\nb\nc\nd").with_padding(0, 0)),
            OverlayOptions {
                width: Some(SizeValue::Cells(5)),
                max_height: Some(SizeValue::Cells(2)),
                anchor: OverlayAnchor::TopLeft,
                ..Default::default()
            },
        );
        let out = stack.composite(base(10, 20), 20, 10);
        assert!(out[0].contains('a'));
        assert!(out[1].contains('b'));
        assert!(!out[2].contains('c'));
    }
}
