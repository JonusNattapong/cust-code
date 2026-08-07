# Tool contracts and event streams

Two contracts everything else hangs off. mistral-vibe's ADRs 0003 and 0004 are the clearest
statements in the survey.

## Tools are typed, permissioned ports (vibe ADR 0004)

> Tools are typed, permissioned ports into side effects. A tool has Pydantic args, result,
> config, and state types, and runs through `BaseTool`. **Permission policy is part of the
> tool contract.**

Four typed slots, not one: **args / result / config / state**. Most designs get args and
result and then smuggle config and state in through closures or globals.

Rationale, stated well:

> Tools are the highest-risk extension point. Typed contracts make LLM calls, validation, UI
> display, session logging, ACP translation, and tests agree on one shape.

### Guidance worth keeping

- Raise a distinct error type for user-facing failures vs authorization failures
  (`ToolError` vs `ToolPermissionError`).
- Keep permission resolution **close to the tool behavior it protects**.
- Keep tool output bounded and safe for LLM context, logs, and transcripts.
- **Do not switch on individual tool names** in the app server or TUI. Public effect kinds
  are *semantic presentation categories*, not a registry of tool names — so arbitrary MCP,
  connector, custom, and future tools render through the generic projection with no dispatch
  code added.

That last one is the difference between a UI that supports MCP tools and one that has a
`switch` statement someone has to extend forever.

### One effect entry per call

> One public app-server effect entry represents the tool call, streaming output, approval
> blocking, result, duration, and terminal state.

Approvals are projected as typed callback entries **related to the same effect** — not as UI
callbacks installed on the tool or the loop. Keeping approval inside the effect model is
what lets a headless client, an editor, and a TUI all handle approval without bespoke code.

### Client-hosted execution

Tools that need the *client's* filesystem or terminal use a `ToolIOPort`; the server sends
typed `clientTool/*` requests. Crucially, the server still owns validation, permissions,
lifecycle, public effects, and model-visible results. The client is an executor, not an
authority. Relevant for remote/SSH/editor-hosted setups.

## Events are the contract (vibe ADR 0003)

> The agent loop communicates through typed events and streaming async generators. It owns
> model and tool execution. Events are the contract for assistant output, reasoning, user
> messages, tool calls, tool streams, tool results, approvals, compaction, plan review,
> title updates, hooks, and related lifecycle changes.

Rules that matter:

- Prefer **adding a typed event** over adding a surface-specific callback into the loop.
- Keep payloads small, serializable, and meaningful **outside one UI**.
- **"Do not make consumers inspect private agent-loop state to understand what happened."**
- Public event IDs are **monotonic**; duplicates are ignored and a **gap is recovered by
  replacing the client projection from a re-read**, not by patching locally.
- Long-running work stays cancellable, and cancellation is visible through the existing
  event/result paths.
- Explicit `create_task` + queues over broad `gather` calls, so orchestration is not hidden.

Flag-to-user conditions: a feature that needs the UI to read or mutate private loop state; an
event useful to one surface but impossible for another to consume; a change that delays
visible feedback until a whole turn completes.

## Tool result shape

clew's contract, simple and proven: `{ ok: boolean, summary: string, data?: unknown }`.

Combine with vibe's typing and you get the target for `cust`:

```rust
trait Tool {
    type Args: DeserializeOwned + JsonSchema;
    type Output: Serialize;
    type Config;
    type State;

    fn permission(&self, args: &Self::Args) -> PermissionRequest;
    fn call(&self, args: Self::Args, ctx: &mut Ctx) -> Result<ToolResult<Self::Output>, ToolError>;
}
```

`permission()` on the trait — not a check the caller remembers to run — is the whole point of
"permission is part of the contract."

## System reminders / context injection is a subsystem

Underappreciated. codex has ~35 files under `core/src/context/` that do nothing but inject
context:

`current_time_reminder`, `token_budget_context`, `turn_aborted`, `subagent_notification`,
`permissions_instructions`, `environment_context`, `model_switch_instructions`,
`plugin_instructions`, `network_rule_saved`, `approved_command_prefix_saved`,
`image_resize_notice`, `guardian_followup_review_reminder`, `multi_agent_mode_instructions`,
`rollout_budget`, `user_instructions`, `legacy_*_warning`, …

grok has `system_reminder.rs` in the agent crate and a `reminders/` directory in tools. clew
injects them too.

These are the channel for "a file changed under you", "you have unread messages", "you are
near the token budget", "the turn was aborted", "the model changed mid-session". Design it as
a registry of typed injections with clear ordering and dedup rules — not string concatenation
at the top of the prompt builder.

## Tool taxonomy

grok's `xai-grok-tools` has `tool_taxonomy.rs`, `normalization.rs`, `attribution.rs`,
`versions.rs`, `persistence.rs`, `retry.rs`, and a `registry/` — tools are a managed
population with versioning, normalization, and retry policy, not a `HashMap<String, fn>`.

Its implementations directory also records something honest: subdirectories named `codex` and
`opencode`, with third-party notices — **they ported tool implementations from other agents
rather than rewriting them.** Reusing a proven `apply_patch` or search tool is a legitimate
move, given attribution.
