use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HistoryItem {
    User(String),
    Assistant(String),
    ToolCall {
        name: String,
        args: serde_json::Value,
    },
    ToolResult {
        name: String,
        summary: String,
    },
}

impl HistoryItem {
    pub fn role_tag(&self) -> &'static str {
        match self {
            Self::User(_) => "user",
            Self::Assistant(_) => "assistant",
            Self::ToolCall { .. } => "tool_call",
            Self::ToolResult { .. } => "tool_result",
        }
    }

    fn text_content(&self) -> &str {
        match self {
            Self::User(t) | Self::Assistant(t) => t.as_str(),
            Self::ToolCall { name, .. } => name.as_str(),
            Self::ToolResult { summary, .. } => summary.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionSummary {
    pub goal: String,
    pub constraints: Vec<String>,
    pub progress: Vec<String>,
    pub key_decisions: Vec<String>,
    pub next_steps: Vec<String>,
    pub modified_files: HashSet<String>,
}

impl CompactionSummary {
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            goal: goal.into(),
            constraints: Vec::new(),
            progress: Vec::new(),
            key_decisions: Vec::new(),
            next_steps: Vec::new(),
            modified_files: HashSet::new(),
        }
    }

    pub fn to_skeleton_markdown(&self) -> String {
        let files = if self.modified_files.is_empty() {
            "None".to_string()
        } else {
            self.modified_files
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        };

        format!(
            "### Context Summary\n\
             - **Goal:** {}\n\
             - **Constraints:** {}\n\
             - **Progress:** {}\n\
             - **Key Decisions:** {}\n\
             - **Next Steps:** {}\n\
             - **Modified Files:** {}\n",
            self.goal,
            if self.constraints.is_empty() {
                "None".to_string()
            } else {
                self.constraints.join("; ")
            },
            if self.progress.is_empty() {
                "None".to_string()
            } else {
                self.progress.join("; ")
            },
            if self.key_decisions.is_empty() {
                "None".to_string()
            } else {
                self.key_decisions.join("; ")
            },
            if self.next_steps.is_empty() {
                "None".to_string()
            } else {
                self.next_steps.join("; ")
            },
            files
        )
    }
}

// ---------------------------------------------------------------------------
// Protected Region Configuration (inspired by hermes-agent)
// ---------------------------------------------------------------------------

/// Controls which head turns to protect during middle-region compression.
#[derive(Debug, Clone)]
pub struct ProtectionPolicy {
    /// Protect the first `User` turn.
    pub protect_first_user: bool,
    /// Protect the first `Assistant` turn.
    pub protect_first_assistant: bool,
    /// Protect the first `ToolCall` turn.
    pub protect_first_tool_call: bool,
    /// Protect the first `ToolResult` turn.
    pub protect_first_tool_result: bool,
    /// Number of tail turns to protect (counted from the end).
    pub protect_last_n: usize,
}

impl Default for ProtectionPolicy {
    fn default() -> Self {
        Self {
            protect_first_user: true,
            protect_first_assistant: true,
            protect_first_tool_call: true,
            protect_first_tool_result: true,
            protect_last_n: 4,
        }
    }
}

/// Result of identifying which indices are compressible.
#[derive(Debug, Clone)]
pub struct ProtectedRegion {
    /// Set of indices that must NOT be compressed.
    pub protected_indices: HashSet<usize>,
    /// First index of the compressible middle region (inclusive).
    pub compress_start: usize,
    /// One-past-the-last index of the compressible middle region (exclusive).
    pub compress_end: usize,
}

pub struct Compactor;

impl Compactor {
    pub fn estimate_tokens(text: &str) -> usize {
        // Rough token estimation: ~4 chars per token
        text.len().div_ceil(4)
    }

    pub fn find_safe_cut_point(items: &[HistoryItem], keep_tokens: usize) -> usize {
        let mut total_tokens = 0;
        let mut cut_idx = 0;

        for (idx, item) in items.iter().enumerate().rev() {
            total_tokens += Self::estimate_tokens(item.text_content());
            if total_tokens >= keep_tokens {
                cut_idx = idx;
                break;
            }
        }

        // Adjust cut index to ensure we don't cut between ToolCall and ToolResult
        if cut_idx < items.len() {
            if let HistoryItem::ToolResult { .. } = &items[cut_idx] {
                // Move back past the ToolCall
                while cut_idx > 0 {
                    if let HistoryItem::ToolCall { .. } = &items[cut_idx - 1] {
                        cut_idx -= 1;
                        break;
                    }
                    cut_idx -= 1;
                }
            }
        }

        cut_idx
    }

    // -----------------------------------------------------------------------
    // Middle-Region Compression (inspired by hermes-agent trajectory compressor)
    // -----------------------------------------------------------------------

    /// Identify protected head/tail indices and the compressible middle region.
    pub fn find_protected_region(
        items: &[HistoryItem],
        policy: &ProtectionPolicy,
    ) -> ProtectedRegion {
        let n = items.len();
        let mut protected = HashSet::new();

        // Track first occurrences of each role
        let mut first_user: Option<usize> = None;
        let mut first_assistant: Option<usize> = None;
        let mut first_tool_call: Option<usize> = None;
        let mut first_tool_result: Option<usize> = None;

        for (i, item) in items.iter().enumerate() {
            match item {
                HistoryItem::User(_) if first_user.is_none() => first_user = Some(i),
                HistoryItem::Assistant(_) if first_assistant.is_none() => first_assistant = Some(i),
                HistoryItem::ToolCall { .. } if first_tool_call.is_none() => {
                    first_tool_call = Some(i)
                }
                HistoryItem::ToolResult { .. } if first_tool_result.is_none() => {
                    first_tool_result = Some(i)
                }
                _ => {}
            }
        }

        // Protect first occurrences
        if policy.protect_first_user {
            if let Some(i) = first_user {
                protected.insert(i);
            }
        }
        if policy.protect_first_assistant {
            if let Some(i) = first_assistant {
                protected.insert(i);
            }
        }
        if policy.protect_first_tool_call {
            if let Some(i) = first_tool_call {
                protected.insert(i);
            }
        }
        if policy.protect_first_tool_result {
            if let Some(i) = first_tool_result {
                protected.insert(i);
            }
        }

        // Protect last N turns
        let tail_start = n.saturating_sub(policy.protect_last_n);
        for i in tail_start..n {
            protected.insert(i);
        }

        // Determine compressible middle region
        let head_protected: Vec<usize> = protected.iter().copied().filter(|i| *i < n / 2).collect();
        let tail_protected: Vec<usize> =
            protected.iter().copied().filter(|i| *i >= n / 2).collect();

        let compress_start = head_protected
            .iter()
            .copied()
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        let compress_end = tail_protected.iter().copied().min().unwrap_or(n);

        // Snap start boundary so it never lands on an orphaned ToolResult
        let compress_start =
            Self::snap_boundary(items, compress_start, compress_start, compress_end);

        ProtectedRegion {
            protected_indices: protected,
            compress_start,
            compress_end,
        }
    }

    /// Check whether `idx` is a clean compression boundary (i.e. does not land
    /// on a `ToolResult` whose preceding `ToolCall` would be on the other side).
    pub fn is_boundary_clean(items: &[HistoryItem], idx: usize) -> bool {
        if idx >= items.len() {
            return true;
        }
        !matches!(&items[idx], HistoryItem::ToolResult { .. })
    }

    /// Move a compression boundary to the nearest clean position.
    ///
    /// Prefers moving forward (folding the orphaned `ToolResult` into the
    /// compressed region alongside its `ToolCall`). Falls back to moving
    /// backward if no clean forward position exists within `[min_idx, max_idx]`.
    pub fn snap_boundary(
        items: &[HistoryItem],
        idx: usize,
        min_idx: usize,
        max_idx: usize,
    ) -> usize {
        // Try forward first
        let mut forward = idx;
        while forward < max_idx && !Self::is_boundary_clean(items, forward) {
            forward += 1;
        }
        if Self::is_boundary_clean(items, forward) {
            return forward;
        }

        // Fall back to backward
        let mut backward = idx;
        while backward > min_idx && !Self::is_boundary_clean(items, backward) {
            backward -= 1;
        }
        backward
    }

    /// Perform middle-region compression: returns indices of items to replace
    /// with a single summary turn. Returns `None` if no compression is needed.
    ///
    /// `target_tokens` — the desired total token count after compression.
    /// `summary_budget` — estimated tokens the replacement summary will consume.
    pub fn plan_middle_compression(
        items: &[HistoryItem],
        target_tokens: usize,
        summary_budget: usize,
        policy: &ProtectionPolicy,
    ) -> Option<std::ops::Range<usize>> {
        let turn_tokens: Vec<usize> = items
            .iter()
            .map(|i| Self::estimate_tokens(i.text_content()))
            .collect();
        let total: usize = turn_tokens.iter().sum();

        if total <= target_tokens {
            return None; // Already fits
        }

        let region = Self::find_protected_region(items, policy);
        if region.compress_start >= region.compress_end {
            return None; // Nothing to compress
        }

        let tokens_to_save = total.saturating_sub(target_tokens);
        // We need to accumulate enough turns so that removing them and
        // inserting the summary achieves the savings.
        // savings = sum(removed_turns) - summary_budget >= tokens_to_save

        let mut accumulated = 0usize;
        let mut compress_up_to = region.compress_start;

        for (i, &tok_count) in turn_tokens
            .iter()
            .enumerate()
            .take(region.compress_end)
            .skip(region.compress_start)
        {
            accumulated += tok_count;
            compress_up_to = i + 1;

            if accumulated.saturating_sub(summary_budget) >= tokens_to_save {
                break;
            }
        }

        // Snap the end boundary so we don't orphan a ToolResult
        let compress_up_to = Self::snap_boundary(
            items,
            compress_up_to,
            region.compress_start,
            region.compress_end,
        );

        if compress_up_to <= region.compress_start {
            return None;
        }

        Some(region.compress_start..compress_up_to)
    }
}
