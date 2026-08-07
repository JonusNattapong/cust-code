//! The original declarative node tree.
//!
//! Predates the [`crate::ink::Component`] port and is kept because existing
//! call sites render static trees to a string. New UI should use `Component`;
//! this stays for the one-shot, non-interactive cases it already serves.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InkColor {
    Default,
    Cyan,
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InkNode {
    Box {
        title: Option<String>,
        color: InkColor,
        children: Vec<InkNode>,
    },
    Text {
        content: String,
        color: InkColor,
        bold: bool,
    },
    Gauge {
        label: String,
        percent: u16,
    },
    Banner {
        art: String,
    },
}

pub struct InkRenderer;

impl InkRenderer {
    pub fn render_to_string(node: &InkNode) -> String {
        match node {
            InkNode::Box { title, children, .. } => {
                let mut out = String::new();
                match title {
                    Some(t) => out.push_str(&format!("┌─ {t} ──────────────────────────┐\n")),
                    None => out.push_str("┌──────────────────────────────────┐\n"),
                }
                for child in children {
                    for line in Self::render_to_string(child).lines() {
                        out.push_str(&format!("│ {line} │\n"));
                    }
                }
                out.push_str("└──────────────────────────────────┘\n");
                out
            }
            InkNode::Text { content, .. } => content.clone(),
            InkNode::Gauge { label, percent } => {
                let width = 20usize;
                // Clamp: an out-of-range percent is a caller bug, not a reason
                // to underflow the empty-segment count.
                let filled = ((width * (*percent as usize)) / 100).min(width);
                let bar = "█".repeat(filled) + &"░".repeat(width - filled);
                format!("{label} [{bar}] {percent}%")
            }
            InkNode::Banner { art } => art.clone(),
        }
    }
}
