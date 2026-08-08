//! Fuzzy completion popup: slash commands, `@file` paths, and skills.
//!
//! Port of the essentials of pi-tui's `autocomplete.ts` (~790 lines): a
//! trigger character selects a provider, the text after it is the query, and
//! [`crate::ink::fuzzy`] ranks the candidates. Rendered as a small overlay
//! panel via [`crate::ink::components::SelectList`]-style rows.
//!
//! Deferred from the full port: async providers, result caching keyed by
//! query prefix, and the editor-side trigger detection (deciding *when* to
//! open the popup as the user types) — this module is the ranking and
//! rendering half; wiring it to live cursor position is future work.

use crate::ink::fuzzy::fuzzy_filter_scored;
use crate::ink::utils::{pad_to_width, truncate_to_width, visible_width};
use crate::ink::Component;

/// One candidate in a completion popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutocompleteItem {
    /// What gets inserted when this item is chosen.
    pub value: String,
    /// What's shown as the primary label.
    pub label: String,
    /// Dimmed detail shown after the label when there's room.
    pub description: Option<String>,
}

impl AutocompleteItem {
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

/// A source of completion candidates for one trigger character.
pub trait AutocompleteProvider {
    /// The character that opens this provider (e.g. `/` for commands, `@`
    /// for file paths).
    fn trigger(&self) -> char;

    /// All candidates this provider can offer. Filtering by query happens in
    /// [`Autocomplete`], not here — providers just supply the universe.
    fn items(&self) -> Vec<AutocompleteItem>;
}

/// A static list of slash commands.
pub struct SlashCommandProvider {
    items: Vec<AutocompleteItem>,
}

impl SlashCommandProvider {
    pub fn new(items: Vec<AutocompleteItem>) -> Self {
        Self { items }
    }
}

impl AutocompleteProvider for SlashCommandProvider {
    fn trigger(&self) -> char {
        '/'
    }

    fn items(&self) -> Vec<AutocompleteItem> {
        self.items.clone()
    }
}

/// File paths for `@file` mentions, supplied by a caller-provided lister so
/// this stays filesystem-agnostic and testable.
pub struct FilePathProvider {
    paths: Vec<String>,
}

impl FilePathProvider {
    pub fn new(paths: Vec<String>) -> Self {
        Self { paths }
    }
}

impl AutocompleteProvider for FilePathProvider {
    fn trigger(&self) -> char {
        '@'
    }

    fn items(&self) -> Vec<AutocompleteItem> {
        self.paths.iter().map(|p| AutocompleteItem::new(p.clone(), p.clone())).collect()
    }
}

/// Dispatches to whichever registered provider matches a trigger character.
#[derive(Default)]
pub struct CombinedAutocompleteProvider {
    providers: Vec<Box<dyn AutocompleteProvider>>,
}

impl CombinedAutocompleteProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Box<dyn AutocompleteProvider>) {
        self.providers.push(provider);
    }

    /// Items for the provider registered to `trigger`, or empty if none.
    pub fn items_for(&self, trigger: char) -> Vec<AutocompleteItem> {
        self.providers
            .iter()
            .find(|p| p.trigger() == trigger)
            .map(|p| p.items())
            .unwrap_or_default()
    }

    pub fn triggers(&self) -> Vec<char> {
        self.providers.iter().map(|p| p.trigger()).collect()
    }
}

/// The completion popup: a ranked, scrollable, keyboard-driven list.
///
/// Not itself a [`crate::ink::Container`] child by default — callers show it
/// via [`crate::ink::Tui::show_overlay`] once a trigger is detected, and hide
/// it on commit/cancel/no-match.
pub struct Autocomplete {
    all_items: Vec<AutocompleteItem>,
    query: String,
    filtered: Vec<AutocompleteItem>,
    selected: usize,
    max_visible: usize,
    submitted: Option<String>,
    cancelled: bool,
}

impl Autocomplete {
    pub fn new(items: Vec<AutocompleteItem>) -> Self {
        let mut a = Self {
            all_items: items,
            query: String::new(),
            filtered: Vec::new(),
            selected: 0,
            max_visible: 8,
            submitted: None,
            cancelled: false,
        };
        a.refilter();
        a
    }

    pub fn with_max_visible(mut self, n: usize) -> Self {
        self.max_visible = n.max(1);
        self
    }

    /// Replace the full candidate set (e.g. when switching providers on a
    /// new trigger character) and re-apply the current query.
    pub fn set_items(&mut self, items: Vec<AutocompleteItem>) {
        self.all_items = items;
        self.refilter();
    }

    /// Update the query text typed after the trigger character.
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.refilter();
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    fn refilter(&mut self) {
        let scored = fuzzy_filter_scored(self.all_items.clone(), &self.query, |item| item.label.clone());
        self.filtered = scored.into_iter().map(|s| s.item).collect();
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
    }

    pub fn has_matches(&self) -> bool {
        !self.filtered.is_empty()
    }

    pub fn selected_item(&self) -> Option<&AutocompleteItem> {
        self.filtered.get(self.selected)
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = if self.selected == 0 {
                self.filtered.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn take_submitted(&mut self) -> Option<String> {
        self.submitted.take()
    }

    pub fn was_cancelled(&self) -> bool {
        self.cancelled
    }
}

impl Component for Autocomplete {
    fn render(&mut self, width: usize) -> Vec<String> {
        if self.filtered.is_empty() {
            return Vec::new();
        }
        let visible_count = self.max_visible.min(self.filtered.len());
        // Keep the selection inside the visible window, matching SelectList.
        let start = if self.selected >= visible_count {
            self.selected + 1 - visible_count
        } else {
            0
        };
        let end = (start + visible_count).min(self.filtered.len());

        let marker = "\u{276f} ";
        let marker_width = visible_width(marker);
        let blank_marker = " ".repeat(marker_width);

        let mut out = Vec::new();
        for (i, item) in self.filtered[start..end].iter().enumerate() {
            let index = start + i;
            let prefix = if index == self.selected { marker } else { &blank_marker };
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
        use crate::ink::keys::{parse_keys, KeyCode};
        for key in parse_keys(data) {
            if key.is_release() {
                continue;
            }
            match key.code {
                KeyCode::Up => self.select_previous(),
                KeyCode::Down => self.select_next(),
                KeyCode::Enter => {
                    self.submitted = self.selected_item().map(|i| i.value.clone());
                }
                KeyCode::Escape => self.cancelled = true,
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

    fn items() -> Vec<AutocompleteItem> {
        vec![
            AutocompleteItem::new("/help", "help").with_description("show help"),
            AutocompleteItem::new("/history", "history"),
            AutocompleteItem::new("/quit", "quit"),
        ]
    }

    #[test]
    fn empty_query_shows_every_item() {
        let a = Autocomplete::new(items());
        assert_eq!(a.filtered.len(), 3);
    }

    #[test]
    fn query_narrows_and_ranks_by_fuzzy_score() {
        let mut a = Autocomplete::new(items());
        a.set_query("hi");
        // "hi" fuzzy-matches "history" but not "help" or "quit".
        assert_eq!(a.filtered.len(), 1);
        assert_eq!(a.selected_item().unwrap().value, "/history");
    }

    #[test]
    fn no_match_renders_nothing() {
        let mut a = Autocomplete::new(items());
        a.set_query("zzz");
        assert!(!a.has_matches());
        assert!(a.render(40).is_empty());
    }

    #[test]
    fn selection_wraps_and_survives_narrowing() {
        let mut a = Autocomplete::new(items());
        a.select_previous();
        assert_eq!(a.selected, 2);
        a.set_query("h"); // narrows to help + history, selection clamps
        assert!(a.selected < a.filtered.len());
    }

    #[test]
    fn window_scrolls_with_the_selection() {
        let many: Vec<_> = (0..20).map(|i| AutocompleteItem::new(format!("v{i}"), format!("item{i}"))).collect();
        let mut a = Autocomplete::new(many).with_max_visible(5);
        for _ in 0..10 {
            a.select_next();
        }
        let out = a.render(20);
        assert_eq!(out.len(), 5);
        assert!(out.iter().any(|l| l.contains("item10")));
    }

    #[test]
    fn enter_submits_the_selected_value() {
        let mut a = Autocomplete::new(items());
        a.handle_input("\r");
        assert_eq!(a.take_submitted(), Some("/help".to_string()));
        assert_eq!(a.take_submitted(), None);
    }

    #[test]
    fn escape_cancels() {
        let mut a = Autocomplete::new(items());
        a.handle_input("\u{1b}");
        assert!(a.was_cancelled());
    }

    #[test]
    fn combined_provider_dispatches_by_trigger_character() {
        let mut combined = CombinedAutocompleteProvider::new();
        combined.register(Box::new(SlashCommandProvider::new(vec![AutocompleteItem::new("/help", "help")])));
        combined.register(Box::new(FilePathProvider::new(vec!["src/main.rs".to_string()])));

        assert_eq!(combined.items_for('/').len(), 1);
        assert_eq!(combined.items_for('@').len(), 1);
        assert!(combined.items_for('#').is_empty());
        assert_eq!(combined.triggers().len(), 2);
    }

    #[test]
    fn file_path_provider_uses_the_path_as_both_value_and_label() {
        let p = FilePathProvider::new(vec!["src/lib.rs".to_string()]);
        let items = p.items();
        assert_eq!(items[0].value, "src/lib.rs");
        assert_eq!(items[0].label, "src/lib.rs");
    }
}
