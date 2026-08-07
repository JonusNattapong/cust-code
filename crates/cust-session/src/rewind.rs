use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewindMode {
    Fork,
    InPlace,
}

impl std::fmt::Display for RewindMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fork => write!(f, "fork"),
            Self::InPlace => write!(f, "in-place"),
        }
    }
}
