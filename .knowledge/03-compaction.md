# Compaction

Every mature agent in the survey has a named compaction component. Three distinct designs.

## prime-agent — batch, best documented

**Trigger:** `contextTokens > contextWindow - reserveTokens`, default `reserveTokens` 16384.
Manual `/compact [instructions]` passes focusing instructions at high priority, persists them
on the entry, and shows them in the TUI.

**Algorithm:**

1. **Find cut point** — walk backwards from newest, accumulating token estimates until
   `keepRecentTokens` (default 20k) is reached.
2. **Extract** messages from the previous kept boundary (or session start) to the cut point.
3. **Summarize** via LLM in a structured format, passing the previous summary as iterative
   context.
4. **Append** a `CompactionEntry` with the summary and `firstKeptEntryId`.
5. **Reload** — session uses summary + messages from `firstKeptEntryId` onward.

```
entry:  0     1     2     3      4     5     6      7      8     9     10
      │ hdr │ usr │ ass │ tool │ usr │ ass │ tool │ tool │ ass │tool│ cmp │
       └────── summarized ─────┘ └──────────── kept ───────────────┘
                                  ↑ firstKeptEntryId

LLM sees:  system │ summary │ usr │ ass │ tool │ tool │ ass │ tool
```

**Cut point rules.** Valid cut points: user messages, assistant messages, BashExecution,
custom messages. **Never cut at a tool result** — it must stay with its tool call.

**Repeated compactions** start the summarized span at the *previous compaction's kept
boundary* (`firstKeptEntryId`), not at the compaction entry — so messages that survived the
earlier pass are included in the next one. `tokensBefore` is recalculated from the rebuilt
context, not carried forward.

**Split turns.** When one turn exceeds `keepRecentTokens`, the cut lands mid-turn at an
assistant message. Then `messagesToSummarize` is empty and a `turnPrefixMessages` list
appears; two summaries are generated (history + turn prefix) and merged. Easy case to miss;
it is exactly the case a long agentic turn produces.

**Summary skeleton** (fixed, and the same one used for branch summaries):

```markdown
## Goal
## Constraints & Preferences
## Progress          ### Done / ### In Progress / ### Blocked
## Key Decisions
## Next Steps
## Critical Context

<read-files>...</read-files>
<modified-files>...</modified-files>
```

File lists accumulate **cumulatively** across compactions — each pass extracts file ops from
the messages being summarized *and* from the previous entry's `details`.

**Serialization.** Before summarizing, messages become text:

```
[User]: what they said
[Assistant thinking]: internal reasoning
[Assistant]: response text
[Assistant tool calls]: ipython(code="..."); edit(path="...")
[Tool result]: output
```

Explicitly "to prevent the model from treating it as a conversation to continue." Tool
results truncated to 2000 chars with a marker saying how much was cut — they are typically
the largest contributor to context.

**Branch summarization.** A second mechanism for `/tree` navigation: find the common
ancestor of old and new positions, collect entries from old leaf back to it, summarize under
a token budget newest-first, append a `BranchSummaryEntry` at the navigation point. Preserves
context from the branch you are abandoning.

**Extensibility.** A `session_before_compact` event can cancel compaction or supply a custom
summary; `details` accepts any JSON-serializable payload. `serializeConversation` and
`convertToLlm` are exported so an extension can summarize with a different model.

## hermes — micro-compaction, amortized

After every normally-finished turn, fold **one** oldest un-absorbed exchange into a running
summary. Same total work, spread out; no single visible stall.

The doc is unusually honest about the cost, and this is the important part:

> Each pass also rewrites already-sent history, which **breaks the provider prompt-cache
> prefix every turn** … for some setups that cost exceeds the benefit.

Also: the turn does not close until the pass finishes, and knowledge becomes second-hand
earlier than under batch compaction — you trade fidelity for a context window that stays
flat instead of sawtoothing.

Off by default (`compression.micro_compact: true`).

**Takeaway: compaction strategy and prompt caching are coupled.** Any design that rewrites
history has to be evaluated against cache-hit economics, not just token counts.

## codex — a family of strategies

`compact.rs`, `compact_remote.rs`, `compact_remote_v2.rs`, `compact_remote_history.rs`,
`compact_remote_request.rs`, `compact_token_budget.rs`, `compact_model_fallback.rs`.
Server-side compaction and a fallback model for the compaction call itself become real
concerns at their scale.

## clew

`services/compact/` plus a separate `services/contextCollapse/`, and per-model `maxContext`
resolution that prefers the provider's live `/models` value over the static table — the
compaction threshold is only as good as the context window number feeding it.

## Design notes for cust

- Batch first (prime-agent's algorithm, including the split-turn case and the
  never-cut-at-a-tool-result rule).
- Fixed summary skeleton with cumulative file tracking.
- Serialize to `[Role]:` lines, truncate tool results.
- Leave a seam for a micro-compaction mode later, but do not enable anything that rewrites
  history until prompt-cache behavior is measured.
- Compaction is **not** a completion signal: it must not stop goals, autonomous
  continuations, heartbeats, or child sessions.
