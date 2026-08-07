# Architecture

`cust` is a **new design**, not a port of clew. Every subsystem below was chosen by taking
the best answer found in the survey ([`.knowledge/`](.knowledge/00-index.md)) rather than by
inheriting one codebase's shape.

This document is the synthesis. `PLAN.md` is the order we build it in.

## What we take from whom

| Subsystem | Source | What specifically |
|---|---|---|
| Code mode | codex + MiMo | JS guest, zero capabilities, **late-bound registry**, cell/wait yield protocol |
| Layering | mistral-vibe | Core engine / composition root / delivery surfaces; ADR format with "Flag To User When" |
| Event model | mistral-vibe | Typed events + streaming, monotonic IDs, gap recovered by re-read |
| Tool contract | mistral-vibe + clew | Typed args/result/config/state, permission **in** the contract; `{ok, summary, data}` result |
| Shell permission | mistral-vibe | Recursive analysis of nested command constructs, not top-level string matching |
| Daemon | prime-agent | Supervisor/worker split, path leases, idempotency journal, generation cursors |
| Scheduling | prime-agent | Claim-then-advance ticks, coalesced missed ticks, per-session job files |
| Session storage | mistral-vibe | Append-friendly + atomic metadata; private format ≠ public projection; two explicit rewind modes |
| Compaction | prime-agent | Cut-point rules, split turns, fixed summary skeleton, cumulative file tracking |
| Sandbox | grok-build | Named profiles, `extends` + glob `deny`, self-protection, fail-closed, honest platform limits |
| Subagents | grok + prime-agent | Agents vs personas; uniform task handle; admission handle, not a blocking call |
| Skills | prime-agent | agentskills.io standard, progressive disclosure, skills-as-libraries once code mode lands |
| Providers | openclaw + clew | Atomic runtime generations; live `/models` preferred over static table |
| Crate seams | codex + grok | config/config-types, tools-api/tools, exec family, git as a subsystem |
| Style rules | codex | Module size limits, exhaustive match, RPITIT over `async_trait`, no bool positional params |
| Editor protocol | kimi/vibe/grok/openclaw | ACP |

Deliberately **not** taken: codex's ~120-crate Bazel scale, hermes' flat 200-module package,
clew's single `src/` with 78 tools and a permanently red typecheck gate.

---

## The shape

```
                      ┌──────────── delivery surfaces ────────────┐
                      │   TUI      ACP      print/JSON/RPC        │
                      └───────────────────┬───────────────────────┘
                                          │  public API: typed commands in, typed events out
                      ┌───────────────────▼───────────────────────┐
                      │            cust-core (composition root)   │
                      │   turn loop · queue · compaction ·        │
                      │   reminders · goal/autonomy policy        │
                      └──┬──────────┬──────────┬──────────┬───────┘
                         │          │          │          │
                  ┌──────▼───┐ ┌────▼─────┐ ┌──▼──────┐ ┌─▼────────┐
                  │ provider │ │  tools   │ │  exec   │ │ session  │
                  │ catalog  │ │ registry │ │ sandbox │ │ store    │
                  └──────────┘ └────┬─────┘ └─────────┘ └──────────┘
                                    │ late-bound, already filtered
                              ┌─────▼──────┐
                              │  codemode  │  QuickJS guest, no capabilities
                              └────────────┘
```

Three rules that make the picture true rather than decorative:

1. **A delivery surface never touches the loop.** It sends typed commands and renders typed
   events. No surface imports `cust-core` internals; no core type imports ratatui or ACP.
2. **The interpreter has no capabilities.** It calls back out through the same filtered tool
   instances the model was offered. It cannot widen its own permissions.
3. **Nothing per-session lives in a global.** Provider/model state is an immutable snapshot
   that each run forks from.

---

## Core contracts

Fix these before features. Everything else hangs off them.

### Tool

```rust
pub trait Tool: Send + Sync {
    type Args: DeserializeOwned + JsonSchema + Send;
    type Output: Serialize + Send;

    fn name(&self) -> &ToolName;
    fn spec(&self) -> &ToolSpec;               // description + schema, for prompt or ALL_TOOLS

    /// What this call needs permission to do. Computed from the args, before execution.
    fn permission(&self, args: &Self::Args) -> PermissionRequest;

    /// Whether this tool may be called from inside code mode.
    fn availability(&self) -> Availability;    // Direct | CodeMode | Both

    fn call(
        &self,
        args: Self::Args,
        cx: &mut ToolCx<'_>,
    ) -> impl Future<Output = Result<ToolResult<Self::Output>, ToolError>> + Send;
}

pub struct ToolResult<T> {
    pub ok: bool,
    pub summary: String,     // one line, model-facing and UI-facing
    pub data: Option<T>,
}

pub enum ToolError {
    Denied(PermissionDenial),   // authorization — distinct on purpose
    Failed(anyhow::Error),      // user-facing failure
}
```

`permission()` on the trait, not at the callsite, is the whole point. `Availability` is how
control-flow tools stay out of the guest without a name blocklist somewhere else.

### Events

One stream, typed, monotonic. The surfaces' only input.

```rust
pub struct Event { pub id: EventId, pub generation: Generation, pub kind: EventKind }

pub enum EventKind {
    TurnStarted { turn: TurnId },
    AssistantDelta { text: String },
    ReasoningDelta { text: String },
    ToolCall   { effect: EffectId, tool: ToolName, args: Value },
    ToolStream { effect: EffectId, chunk: String },
    ToolResult { effect: EffectId, ok: bool, summary: String },
    ApprovalRequested { effect: EffectId, request: PermissionRequest },
    Compacted  { entry: EntryId, tokens_before: u32, tokens_after: u32 },
    Reminder   { kind: ReminderKind },
    TurnEnded  { turn: TurnId, reason: EndReason },
    Error      { recoverable: bool, message: String },
}
```

`EffectId` ties call, stream, approval, and result into **one effect** — so approval works
identically in the TUI, over ACP, and headless. Event kinds are *semantic categories*; a UI
must never switch on tool name.

### Session

Two contracts, never conflated:

- **Storage** — JSONL, append-only for messages, atomic writes for metadata, versioned for
  migration. Only the session layer reads or writes it.
- **Projection** — `PublicSessionState`, lossy, paged through opaque cursors. *Public events
  are not a persistence format.*

Rewind is two explicit modes (`Fork` | `InPlace`); the destructive one is never inferred from
a missing option.

### Permission

```rust
pub enum PermissionRequest {
    None,
    ReadPath(PathBuf),
    WritePath(PathBuf),
    Execute(ShellPlan),      // parsed, recursively — not a command string
    Network(Url),
    Custom(String),
}
```

`ShellPlan` is the parse tree, so nested constructs (`sh -c`, pipelines, `xargs`, backticks,
command substitution) are analyzed, and paths are normalized — including MSYS drive forms on
Windows — before any workspace-boundary check.

---

## Crates

| Crate | Owns | Must not depend on |
|---|---|---|
| `cust-config-types` | config types only | anything |
| `cust-tools-api` | `Tool`, `ToolResult`, `PermissionRequest`, specs | tool impls, core |
| `cust-proto` | ACP + daemon wire types | core, tui |
| `cust-skill` | progressive disclosure `SKILL.md` & script skills | core, tui |
| `cust-config` | loading, layering, profiles | core, tools |
| `cust-provider` | catalog, generations, streaming, auth, usage | core, tools |
| `cust-session` | transcript store, leases, rewind, projection | tui, proto surfaces |
| `cust-exec` | shell, PTY, sandbox profiles | core |
| `cust-tools` | implementations + late-bound registry | tui, core internals |
| `cust-codemode` | QuickJS guest + engine-agnostic host bridge | tools impls (uses the registry) |
| `cust-core` | turn loop, events, compaction, reminders, policy | tui, acp, any surface |
| `cust-tui` | ratatui rendering | — (consumes public API only) |
| `cust-code` | binary: args, composition root | — |

The dependency rule in one line: **surfaces depend on core; core depends on capability
crates; capability crates depend on `*-types` and `*-api`. Never the reverse.**

---

## Resolved: the three open questions

### 1. Rewrite of clew, companion, or new? → **New design**

Informed by everything, inheriting nothing. Consequences:

- No requirement to read clew's session format. We may write an importer later; it is not a
  constraint on the design.
- Credentials are the one deliberate reuse — read-only, because re-authenticating every
  provider is pure friction with no design cost.
- clew's *lessons* carry over (tool result shape, live context windows, the
  process-global-provider trap); clew's *structure* does not.

### 2. Code mode only, or direct tools too? → **Both**

codex and MiMo both ship a small direct set alongside `exec`, and they are right: the model
should not have to write a script to read one file.

- **Direct:** `bash`, `read`, `edit`, `exec` (code mode), `view_image`.
- **Code mode only:** everything composable — search, multi-file reads, batch edits,
  analysis pipelines, MCP tool fan-out.
- **Neither:** control-flow tools (`task`, `session`, `cron`, `skill`) stay outside the guest.

Enforced by `Tool::availability()`, so the profile is a property of each tool rather than a
list maintained in two places. MiMo's unfinished edge — prompt routing and tool profiles
driven by two separate sets of model-ID string rules — is the thing to avoid: **one
capability negotiation, consulted by both.**

### 3. Subagents inside code mode? → **No at Phase 4, yes by design at Phase 9**

MiMo's rule applies while the foundation is thin: spawning changes scheduling state and
should not hide inside a script call. But prime-agent's `rlm()` is genuinely more expressive,
and we want it eventually.

So: build the host bridge **engine-agnostic and typed from the start** — the guest sends
`HostRequest`, the host validates and owns every state transition. At Phase 4 the only
request kinds are tool calls. At Phase 9 we add `spawn_child` behind a depth limit, and the
transport does not change.

The trap to avoid is prime-agent's deadlock: a guest awaiting admission cannot be answered on
the same channel it is blocking. Design the bridge with a **separate reply path** for host
requests before anything awaits.

---

## Non-goals, for now

Multi-agent swarms, LAN peers, voice, chat-channel delivery, image/video generation, a
plugin marketplace. Each exists in some surveyed project and each is real work; none of them
teaches us anything about whether the core contracts are right. Revisit after Phase 5.
