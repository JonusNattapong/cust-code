use crate::ink::keys::{parse_keys, KeyCode};
use crate::ink::utils::{pad_to_width, truncate_to_width};
use crate::ink::Component;

/// One row in a [`SelectList`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectItem {
    /// Stable identifier returned on selection.
    pub value: String,
    /// Primary label shown on the row.
    pub label: String,
    /// Dimmed detail shown after the label when there is room.
    pub description: Option<String>,
}

impl SelectItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// A scrollable, keyboard-driven single-selection list.
///
/// Port of `pi-tui`'s `SelectList`. The visible window follows the selection
/// so the cursor never leaves the screen, and the list wraps at both ends.
pub struct SelectList {
    items: Vec<SelectItem>,
    selected: usize,
    /// Index of the first visible row.
    scroll_top: usize,
    max_visible: usize,
    marker: String,
    /// Set once the user commits a choice; read and cleared by the caller.
    submitted: Option<String>,
    cancelled: bool,
}

impl SelectList {
    pub fn new(items: Vec<SelectItem>) -> Self {
        Self {
            items,
            selected: 0,
            scroll_top: 0,
            max_visible: 10,
            marker: "❯ ".to_string(),
            submitted: None,
            cancelled: false,
        }
    }

    pub fn with_max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible.max(1);
        self
    }

    pub fn set_items(&mut self, items: Vec<SelectItem>) {
        self.items = items;
        self.selected = 0;
        self.scroll_top = 0;
    }

    pub fn items(&self) -> &[SelectItem] {
        &self.items
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&SelectItem> {
        self.items.get(self.selected)
    }

    /// The value chosen since the last call, if any. Consumes the result.
    pub fn take_submitted(&mut self) -> Option<String> {
        self.submitted.take()
    }

    pub fn was_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.items.len();
        self.ensure_visible();
    }

    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.items.len() - 1
        } else {
            self.selected - 1
        };
        self.ensure_visible();
    }

    /// Scroll the window so the selected row is inside it.
    fn ensure_visible(&mut self) {
        if self.selected < self.scroll_top {
            self.scroll_top = self.selected;
        } else if self.selected >= self.scroll_top + self.max_visible {
            self.scroll_top = self.selected + 1 - self.max_visible;
        }
    }
}

impl Component for SelectList {
    fn render(&mut self, width: usize) -> Vec<String> {
        if self.items.is_empty() {
            return Vec::new();
        }
        let end = (self.scroll_top + self.max_visible).min(self.items.len());
        let marker_width = crate::ink::utils::visible_width(&self.marker);
        let blank_marker = " ".repeat(marker_width);

        let mut out = Vec::new();
        for (i, item) in self.items[self.scroll_top..end].iter().enumerate() {
            let index = self.scroll_top + i;
            let prefix = if index == self.selected {
                &self.marker
            } else {
                &blank_marker
            };
            let body = match &item.description {
                Some(d) => format!("{}  {}", item.label, d),
                None => item.label.clone(),
            };
            let budget = width.saturating_sub(marker_width);
            let clipped = truncate_to_width(&body, budget, Some("…"));
            out.push(pad_to_width(&format!("{prefix}{}", clipped.text), width));
        }
        out
    }

    fn handle_input(&mut self, data: &str) {
        for key in parse_keys(data) {
            if key.is_release() {
                continue;
            }
            match key.code {
                KeyCode::Up => self.select_previous(),
                KeyCode::Down => self.select_next(),
                KeyCode::Char('k') if !key.modifiers.ctrl => self.select_previous(),
                KeyCode::Char('j') if !key.modifiers.ctrl => self.select_next(),
                KeyCode::Enter => {
                    self.submitted = self.selected_item().map(|i| i.value.clone());
                }
                KeyCode::Escape => self.cancelled = true,
                KeyCode::Char('c') if key.modifiers.ctrl => self.cancelled = true,
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
    use crate::ink::utils::visible_width;

    fn list(n: usize) -> SelectList {
        SelectList::new(
            (0..n)
                .map(|i| SelectItem::new(format!("v{i}"), format!("label{i}")))
                .collect(),
        )
    }

    #[test]
    fn empty_list_renders_nothing() {
        assert!(SelectList::new(vec![]).render(20).is_empty());
    }

    #[test]
    fn marks_the_selected_row_only() {
        let mut l = list(3);
        let out = l.render(20);
        assert!(out[0].starts_with('❯'));
        assert!(out[1].starts_with("  "));
    }

    #[test]
    fn selection_wraps_at_both_ends() {
        let mut l = list(3);
        l.select_previous();
        assert_eq!(l.selected_index(), 2);
        l.select_next();
        assert_eq!(l.selected_index(), 0);
    }

    #[test]
    fn window_follows_the_selection_downward() {
        let mut l = list(10).with_max_visible(3);
        for _ in 0..3 {
            l.select_next();
        }
        let out = l.render(20);
        assert_eq!(out.len(), 3);
        assert!(out[2].contains("label3"));
    }

    #[test]
    fn window_follows_the_selection_upward() {
        let mut l = list(10).with_max_visible(3);
        l.select_previous(); // wraps to the last item
        let out = l.render(20);
        assert!(out[2].contains("label9"));
    }

    #[test]
    fn rows_are_clipped_and_padded_to_width() {
        let mut l = SelectList::new(vec![
            SelectItem::new("v", "a rather long label").with_description("and a description")
        ]);
        let out = l.render(15);
        assert_eq!(visible_width(&out[0]), 15);
    }

    #[test]
    fn arrow_and_vim_keys_both_navigate() {
        let mut l = list(3);
        l.handle_input("\u{1b}[B");
        assert_eq!(l.selected_index(), 1);
        l.handle_input("j");
        assert_eq!(l.selected_index(), 2);
        l.handle_input("k");
        assert_eq!(l.selected_index(), 1);
        l.handle_input("\u{1b}[A");
        assert_eq!(l.selected_index(), 0);
    }

    #[test]
    fn enter_submits_the_selected_value() {
        let mut l = list(3);
        l.select_next();
        l.handle_input("\r");
        assert_eq!(l.take_submitted(), Some("v1".to_string()));
        // The result is consumed, not latched.
        assert_eq!(l.take_submitted(), None);
    }

    #[test]
    fn escape_and_ctrl_c_cancel() {
        let mut l = list(3);
        l.handle_input("\u{1b}");
        assert!(l.was_cancelled());

        let mut l = list(3);
        l.handle_input("\u{3}");
        assert!(l.was_cancelled());
    }

    #[test]
    fn key_releases_are_ignored() {
        let mut l = list(3);
        // Kitty release event for 'j' must not move the selection.
        l.handle_input("\u{1b}[106;1:3u");
        assert_eq!(l.selected_index(), 0);
    }

    #[test]
    fn setting_items_resets_position() {
        let mut l = list(5);
        l.select_next();
        l.set_items(vec![SelectItem::new("x", "x")]);
        assert_eq!(l.selected_index(), 0);
    }
}
