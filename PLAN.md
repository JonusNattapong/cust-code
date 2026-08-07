# PLAN

Build order for `cust-code`. [ARCHITECTURE.md](ARCHITECTURE.md) says *what we are building*,
[DESIGN-NOTES.md](DESIGN-NOTES.md) says *why those choices*, this file says *in what order*.

## Working agreement

- **Every phase ships something runnable.** No phase is done until the binary does the new
  thing when a human types it — not just until tests pass.
- **Verify for real, every time.** Each phase lists an explicit smoke test. Run it, paste
  the actual output into the phase's completion note, and report failures as failures.
- **Docs move with the code.** `PLAN.md`, `README.md`, `DESIGN-NOTES.md`, and `CHANGELOG.md`
  get updated in the same commit as the behavior they describe.
- **Gate before commit:** `cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

## Decisions already made

| Question | Decision |
|---|---|
| Relationship to clew | **A new design.** Informed by all nine surveyed agents, inheriting no codebase. clew's lessons carry over; its structure does not. |
| Language | Rust, edition 2024 |
| Package / binary | crate `cust-code`, binary `cust` |
| Code-mode engine | QuickJS via `rquickjs` (start); `deno_core` only if it proves limiting |
| Tool surface | **Both** — a small direct set (`bash`, `read`, `edit`, `exec`, `view_image`) plus code mode for everything composable. Chosen per tool via `Tool::availability()`. |
| Subagents in code mode | Not at Phase 4; by design at Phase 9. The host bridge is typed and engine-agnostic from the start so the transport does not change. |
| Editor protocol | ACP — adopt, don't invent |
| Credentials | Reuse clew's — `~/.clew/.credentials.json`, `~/.clew/provider.json`, project `.env`. Read-only; `cust` never writes to clew's files. |
| Tool result | `{ ok, summary, data? }` |

Reasoning for each is in [.memory/DECISIONS.md](.memory/DECISIONS.md); the synthesis is in
[ARCHITECTURE.md](ARCHITECTURE.md).

## Still open

Nothing blocking. Two things to settle when we reach them:

1. `~/.cust/` or `~/.config/cust/` for our own state (Phase 1).
2. Whether to ship a clew session importer at all (revisit after Phase 5).

---

## Phase 0 — Scaffold ✅

Cargo project, `cust help`, `cust version`, design survey written.

**Verified:** `cargo build` clean; `cust` prints help, `cust version` prints `0.0.0`.

---

## Phase 1 — Talk to a model

The smallest thing that is actually an agent: one prompt in, one streamed answer out.

- `cust-config-types` + `cust-config`: layered config (defaults → `~/.cust/config.toml` →
  `./.cust/config.toml` → env → flags).
- `cust-provider`: one provider first (whichever key `~/.clew/.credentials.json` already
  holds), streaming, typed usage. Model capabilities declared as
  `chat`/`vision`/`tool_calling`/`streaming`/`max_context`.
- Credential loader that reads clew's files without mutating them, with a clear error when
  a key is missing rather than a silent fallback.
- `cust ask "<prompt>"` — non-interactive, streams to stdout.

**Smoke test:** `cust ask "reply with the single word: ok"` streams `ok` and exits 0.
Then unset the key and confirm the error names the file it looked in.

## Phase 2 — Tools and the turn loop

- `cust-tools-api`: `Tool` trait with typed args/result and **permission as part of the
  contract** (vibe ADR 0004). Not a bolt-on check at the callsite.
- `cust-tools`: `read_file`, `write_file`, `list_dir`, `search` (ripgrep-style).
- `cust-core`: the turn loop, emitting **typed events** over a stream — assistant text,
  tool call, tool result, error, turn end. Monotonic IDs (vibe ADR 0003). The CLI renders
  events; it never reaches into loop state.
- Approval prompt for writes.

**Smoke test:** `cust "read Cargo.toml and tell me the edition"` returns `2024` having
actually called `read_file`. `cust "delete README.md"` asks before doing anything.

## Phase 3 — Shell and sandbox

- `cust-exec`: command execution, PTY, output capture with bounded size.
- Shell permission analysis that **recurses into nested command constructs** rather than
  matching the top-level string (vibe ADR 0004).
- Named sandbox profiles modeled on grok's: `off` / `workspace` / `read-only` / `strict`,
  with a glob `deny` list. Landlock on Linux, Seatbelt on macOS. **State plainly in the docs
  what is and is not enforced on Windows** rather than implying coverage.

**Smoke test:** under `--sandbox read-only`, a write outside `~/.cust/` fails at the kernel,
not at a Rust `if`. Under `strict`, a read outside CWD fails. Both verified by running the
binary, not by unit test alone.

## Phase 4 — Code-mode

The headline feature. See DESIGN-NOTES for why four independent teams landed here.

- `cust-codemode`: QuickJS guest with **no** filesystem, network, timers, or module loading.
- Host bridge exposing `tools.<name>()` from a **late-bound registry** — the same filtered
  tool instances the outer layer got, so a hidden tool cannot reappear inside a script.
- Control-flow tools excluded from the guest, via `Tool::availability()` — a property of each
  tool, not a blocklist maintained somewhere else.
- **Typed, engine-agnostic host bridge with a separate reply path** for host requests, so
  Phase 9 can add `spawn_child` without changing the transport, and so a guest awaiting a
  request is never blocked on the channel it is waiting on (prime-agent's deadlock).
- Resource limits, MiMo's numbers as the starting point: 50 nested calls, 8 concurrent,
  60s active compute, 64 MiB guest memory, bounded code/return/log sizes.
- Yield protocol: `exec` returns a cell id when still running; `wait(cell_id, …)` pulls new
  output; `yield_control()` flushes early.

**Smoke test:** a script that reads three files and returns one summary completes in **one**
model round-trip. Then confirm a tool excluded by config is genuinely absent from `tools`
inside the guest — this is the security-critical assertion, test it explicitly.

## Phase 5 — Sessions

- `cust-session`: JSONL transcript, append-friendly for messages, atomic for metadata,
  migration-tolerant. Private storage format ≠ public projection (vibe ADR 0006).
- Leases keyed by canonical transcript path; concurrent open returns `session_already_active`
  with the owner's id (prime-agent's daemon model).
- `cust resume`, `cust list`.
- Rewind with two **explicit** modes — fork and in-place. The destructive one is never
  inferred from a missing option.

**Smoke test:** run a session, kill the process, `cust resume` recovers it. Open the same
session twice and confirm the second attempt is refused with the owner's id.

## Phase 6 — Compaction

- Trigger at `context_tokens > context_window - reserve_tokens`.
- Walk back to `keep_recent_tokens`; **never cut at a tool result**; handle the split turn.
- Fixed summary skeleton (Goal / Constraints / Progress / Key Decisions / Next Steps /
  Critical Context) with read/modified files tracked cumulatively.
- Serialize history as `[User]:` / `[Assistant]:` / `[Tool result]:` lines so the model
  summarizes rather than continues; truncate tool results.

**Smoke test:** drive a session past the window and confirm it keeps working, that the
summary names the actual goal, and that the token count actually drops.

## Phase 7 — TUI

- `cust-tui` on ratatui. Renders the Phase 2 event stream; owns no agent state.

**Smoke test:** interactive session with streaming output, interrupt mid-turn, resize.

## Phase 8 — Daemon and ACP

- `cust-proto`: ACP types plus the local daemon protocol.
- Supervisor / worker split — supervisor routes and never executes (prime-agent's model).
- Idempotency journal keyed by `client_id + command_id`, written **before** dispatch; an
  uncertain command is reported uncertain and **not replayed**.
- Generation-aware event cursors `{generation, sequence}`; attach snapshot is the recovery
  baseline.
- Detach and reattach.

**Smoke test:** start a task, close the terminal, reattach and find it still running. Kill
the worker mid-turn and confirm recovery appends a visible marker and does not re-run the
side effect.

## Phase 9 — Subagents

- Child sessions with independent context, linked to the parent transcript.
- One uniform handle for background shell jobs and subagents (grok's model):
  `get_output(task_id)`, `wait(task_ids, mode=any|all)`, `kill(task_id)`.
- Depth limit. Usage attributed to the parent without inflating its context measurement.

**Smoke test:** spawn two subagents in parallel, both report back, parent's own context
does not grow by their transcripts.

---

## Deliberately not in scope yet

Skills, MCP client, hooks, memory, LSP, multi-provider fallback chains, voice, peers. Each
is real work and each is cheaper once the phases above have fixed the contracts they'd hang
off. Revisit after Phase 5.
