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
                if let Some(t) = title {
                    out.push_str(&format!("┌─ {} ──────────────────────────┐\n", t));
                } else {
                    out.push_str("┌──────────────────────────────────┐\n");
                }
                for child in children {
                    let child_str = Self::render_to_string(child);
                    for line in child_str.lines() {
                        out.push_str(&format!("│ {} │\n", line));
                    }
                }
                out.push_str("└──────────────────────────────────┘\n");
                out
            }
            InkNode::Text { content, .. } => content.clone(),
            InkNode::Gauge { label, percent } => {
                let width = 20;
                let filled = (width * (*percent as usize)) / 100;
                let empty = width - filled;
                let bar: String = "█".repeat(filled) + &"░".repeat(empty);
                format!("{} [{}] {}%", label, bar, percent)
            }
            InkNode::Banner { art } => art.clone(),
        }
    }
}
