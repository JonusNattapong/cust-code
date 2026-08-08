//! `ink` — a differential-rendering terminal UI layer.
//!
//! A Rust port of [`pi-tui`][pi-tui], the TUI library behind prime-agent
//! (`@earendil-works/pi-tui`). The model is deliberately small:
//!
//! - A [`Component`] turns a width into lines. That is the whole contract —
//!   no layout engine, no virtual DOM, no reconciler.
//! - A [`Container`] stacks components vertically.
//! - A [`Differ`] compares the new frame against the last one and emits only
//!   the escape sequences needed to reconcile them, so a streaming transcript
//!   repaints the rows that changed instead of replaying from the top.
//! - A [`Tui`] wires those to a [`Terminal`] and routes input to whichever
//!   component holds focus.
//!
//! The diffing core takes no terminal I/O, so it is asserted on directly in
//! tests against [`terminal::TestTerminal`].
//!
//! [pi-tui]: https://github.com/PrimeIntellect-ai/prime-agent/tree/main/packages/tui
//!
//! # Not yet ported
//!
//! The alternate-screen fullscreen viewport, terminal image protocols
//! (Kitty/iTerm2), and LaTeX-to-Unicode.

pub mod autocomplete;
pub mod component;
pub mod components;
pub mod differ;
pub mod fuzzy;
pub mod keys;
pub mod legacy;
pub mod overlay;
pub mod render_cache;
pub mod terminal;
pub mod tui;
pub mod utils;

pub use autocomplete::{
    Autocomplete, AutocompleteItem, AutocompleteProvider, CombinedAutocompleteProvider, EditorAutocomplete,
    FilePathProvider, SlashCommandProvider,
};
pub use component::{Component, Container, CURSOR_MARKER};
pub use components::{
    BoxView, Editor, Loader, Markdown, SelectItem, SelectList, Spacer, StatusLine, Text, TruncatedText,
};
pub use differ::{CursorPos, Differ, Frame, FrameKind, FullRedrawReason};
pub use fuzzy::{fuzzy_filter, fuzzy_filter_scored, fuzzy_match, FuzzyMatch, ScoredItem};
pub use keys::{parse_key, parse_keys, Key, KeyCode, KeyEventType, Modifiers};
pub use overlay::{OverlayAnchor, OverlayId, OverlayMargin, OverlayOptions, OverlayStack, SizeValue};
pub use render_cache::VersionedRenderCache;
pub use terminal::{ProcessTerminal, Terminal, TerminalStopOptions, TestTerminal};
pub use tui::Tui;
pub use utils::{
    strip_ansi, truncate_to_width, visible_width, wrap_text_with_ansi,
};

// The pre-port declarative tree, kept for existing call sites.
pub use legacy::{InkColor, InkNode, InkRenderer};
