//! The built-in component library.
//!
//! Ports of `pi-tui`'s `src/components/*.ts`. Each one is a plain struct
//! implementing [`crate::ink::Component`]: width in, lines out.

mod boxed;
mod columns;
mod editor;
mod loader;
mod markdown;
mod select_list;
mod spacer;
mod status;
mod text;
mod truncated_text;

pub use boxed::{BorderStyle, BoxView};
pub use columns::Columns;
pub use editor::Editor;
pub use loader::Loader;
pub use markdown::Markdown;
pub use select_list::{SelectItem, SelectList};
pub use spacer::Spacer;
pub use status::StatusLine;
pub use text::Text;
pub use truncated_text::TruncatedText;
