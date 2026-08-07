use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EventCursor {
    pub generation: u64,
    pub sequence: u64,
}

impl EventCursor {
    pub fn new(generation: u64, sequence: u64) -> Self {
        Self {
            generation,
            sequence,
        }
    }

    pub fn advance(&mut self) {
        self.sequence += 1;
    }
}
