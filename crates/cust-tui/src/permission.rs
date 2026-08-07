//! Permission modes: how tool requests are answered, and the footer hint that
//! shows the current mode.
//!
//! Shift+Tab cycles the mode. Because approval callbacks run inside the agent
//! loop rather than the draw loop, the live mode is shared through
//! [`SharedPermissionMode`] rather than read off `TuiState`.

use cust_tools_api::PermissionRequest;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// How the TUI answers tool permission requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    /// Deny anything that needs approval. The safe default.
    #[default]
    Ask,
    /// Allow file writes, still refuse shell execution.
    AcceptEdits,
    /// Allow everything without asking.
    BypassPermissions,
    /// Read-only: refuse every mutating request.
    Plan,
}

impl PermissionMode {
    /// Cycle order for Shift+Tab.
    pub fn next(self) -> Self {
        match self {
            Self::Ask => Self::AcceptEdits,
            Self::AcceptEdits => Self::BypassPermissions,
            Self::BypassPermissions => Self::Plan,
            Self::Plan => Self::Ask,
        }
    }

    /// Parse a mode name as typed into `/permissions`.
    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim().to_ascii_lowercase().replace(['_', ' '], "-").as_str() {
            "ask" => Some(Self::Ask),
            "accept-edits" | "edits" | "accept" => Some(Self::AcceptEdits),
            "bypass" | "bypass-permissions" | "yolo" => Some(Self::BypassPermissions),
            "plan" => Some(Self::Plan),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ask => "ask every time",
            Self::AcceptEdits => "accept edits",
            Self::BypassPermissions => "bypass permissions",
            Self::Plan => "plan mode",
        }
    }

    /// The footer line, e.g. `⏵⏵ bypass permissions on (shift+tab to cycle)`.
    pub fn footer(self) -> String {
        format!("⏵⏵ {} on (shift+tab to cycle)", self.label())
    }

    /// Whether a request is granted without asking the user.
    pub fn allows(self, request: &PermissionRequest) -> bool {
        // Reads never mutate anything, so they pass in every mode.
        let read_only = matches!(
            request,
            PermissionRequest::None | PermissionRequest::ReadPath(_)
        );
        match self {
            Self::BypassPermissions => true,
            Self::AcceptEdits => read_only || matches!(request, PermissionRequest::WritePath(_)),
            Self::Ask | Self::Plan => read_only,
        }
    }

    fn from_u8(raw: u8) -> Self {
        match raw {
            1 => Self::AcceptEdits,
            2 => Self::BypassPermissions,
            3 => Self::Plan,
            _ => Self::Ask,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            Self::Ask => 0,
            Self::AcceptEdits => 1,
            Self::BypassPermissions => 2,
            Self::Plan => 3,
        }
    }
}

/// A [`PermissionMode`] shared between the draw loop and the approval callback.
#[derive(Debug, Clone, Default)]
pub struct SharedPermissionMode(Arc<AtomicU8>);

impl SharedPermissionMode {
    pub fn new(mode: PermissionMode) -> Self {
        Self(Arc::new(AtomicU8::new(mode.as_u8())))
    }

    pub fn get(&self) -> PermissionMode {
        PermissionMode::from_u8(self.0.load(Ordering::Relaxed))
    }

    pub fn set(&self, mode: PermissionMode) {
        self.0.store(mode.as_u8(), Ordering::Relaxed);
    }

    /// Advance to the next mode and return it.
    pub fn cycle(&self) -> PermissionMode {
        let next = self.get().next();
        self.set(next);
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycling_visits_every_mode_and_returns_to_the_start() {
        let mut mode = PermissionMode::Ask;
        let mut seen = vec![mode];
        for _ in 0..3 {
            mode = mode.next();
            seen.push(mode);
        }
        assert_eq!(
            seen,
            vec![
                PermissionMode::Ask,
                PermissionMode::AcceptEdits,
                PermissionMode::BypassPermissions,
                PermissionMode::Plan
            ]
        );
        assert_eq!(mode.next(), PermissionMode::Ask);
    }

    #[test]
    fn footer_reads_like_the_status_hint() {
        assert_eq!(
            PermissionMode::BypassPermissions.footer(),
            "⏵⏵ bypass permissions on (shift+tab to cycle)"
        );
    }

    #[test]
    fn modes_gate_requests_by_severity() {
        let write = PermissionRequest::WritePath("a.txt".into());
        let exec = PermissionRequest::Execute("rm -rf /".to_string());

        assert!(!PermissionMode::Ask.allows(&write));
        assert!(!PermissionMode::Ask.allows(&exec));

        assert!(PermissionMode::AcceptEdits.allows(&write));
        assert!(!PermissionMode::AcceptEdits.allows(&exec));

        assert!(PermissionMode::BypassPermissions.allows(&write));
        assert!(PermissionMode::BypassPermissions.allows(&exec));

        assert!(!PermissionMode::Plan.allows(&write));
        assert!(!PermissionMode::Plan.allows(&exec));
    }

    #[test]
    fn shared_mode_is_visible_to_every_holder() {
        let a = SharedPermissionMode::new(PermissionMode::Ask);
        let b = a.clone();
        assert_eq!(b.cycle(), PermissionMode::AcceptEdits);
        assert_eq!(a.get(), PermissionMode::AcceptEdits);
    }
}
