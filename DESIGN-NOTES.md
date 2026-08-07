# Design notes

Prior-art survey done before writing cust's real code. Sources: the eight agents under
`clew-code/.reference/` plus clew-code itself.

Depth: architecture docs, ADRs, and crate/package layouts read in full for each project.
Implementation internals (actual loop code) not read line by line except where quoted.

| Project | Org | Lang | Shape |
|---|---|---|---|
| **codex** | OpenAI | Rust + TS | ~120 crates, Bazel + Cargo |
| **grok-build** | xAI | Rust | ~60 crates, generated workspace root |
| **kimi-cli** | Moonshot | Python | `src/kimi_cli` + `kosong` LLM layer |
| **mistral-vibe** | Mistral | Python | `vibe/core` + surfaces, 10 ADRs |
| **hermes-agent** | Nous | Python + TS | flat `agent/` (~200 modules) |
| **MiMo-Code** | Xiaomi | TS (Bun) | 17 packages, opencode fork |
| **openclaw** | community | TS | 22 packages + gateway |
| **prime-agent** | Prime Intellect | TS | 4 packages, built on `pi` |
| **clew-code** | us | TS (Bun) | one `src/`, 78 tools, 114 commands |

---

## The finding: four teams independently converged on code-as-the-tool-interface

This is the single strongest signal in the whole survey. Four projects, no shared code,
same conclusion — **stop giving the model N tool schemas; give it a sandboxed interpreter
where the tools are functions.**

| Project | Engine | Interface | Session state |
|---|---|---|---|
| codex `code-mode` | **V8 isolate** (fresh per call) | `await tools.exec_command(...)`, `ALL_TOOLS` | `store(k,v)` / `load(k)` |
| MiMo-Code `exec` | **QuickJS** | `tools.<name>()` | per-call |
| prime-agent `ipython` | **IPython kernel** (persistent) | any Python; `rlm()`, skills as imports | full namespace, survives compaction |
| hermes | Python + RPC | scripts call tools over RPC | — |

Why they all did it: a five-step pipeline that costs five model round-trips as tool calls
costs **one** as a script. codex's own tool description says it plainly — "Run JavaScript
code to orchestrate/compose tool calls."

### The three details that make it safe

Not obvious from the outside, and all three appear in more than one project:

1. **The interpreter never gets capabilities — the host does.** codex: "no Node, no file
   system, no network access, no console." MiMo: QuickJS with no `process`/`fetch`/timers/
   module loading. Every real side effect goes back out to a host tool that runs the normal
   permission checks. The sandbox isolates *code*, not *effects*.
2. **Late-bound tool registry.** MiMo's `tool-script-ref.ts` hands `exec` the *same*
   `Tool.Def` instances the outer layer got, after model/agent filtering — so a tool hidden
   from the model cannot reappear inside the script. Without this, code-mode is a permission
   bypass wearing a sandbox costume.
3. **Control-flow tools are excluded.** MiMo keeps `task`, `skill`, `workflow`, `cron`,
   `session`, `question` out of `exec` because they mutate conversation/scheduling state and
   must not hide inside one script call. Note prime-agent deliberately goes the *other* way
   (`rlm()` spawns subagents from inside Python) — but pays for it with a whole typed
   host-request bridge and a depth limit.

### And the yield protocol

Long scripts can't block a turn. codex: `exec` returns `Script running with cell ID ...`,
then `wait(cell_id, yield_time_ms, max_tokens, terminate)` pulls new output; plus
`yield_control()` to flush output while still running, and a `// @exec: {...}` first-line
pragma for per-call budgets. MiMo bounds it with hard numbers: 50 nested calls (max 500),
8 concurrent, 60s active compute, 30min wall clock, 64 MiB guest memory, 128 KiB code /
256 KiB return / 64 KiB logs.

**For cust (Rust), the path is clear** — codex and MiMo both prove the Rust/TS-host +
embedded-JS-guest design. QuickJS via `rquickjs` is the pragmatic start (small, embeddable,
no V8 build); V8 via `deno_core` is the scale answer. Python is *not* the right call for a
Rust binary: prime-agent needs `uv`, a bootstrapped venv, ipykernel, and a ZeroMQ Jupyter
transport to make it work.

---

## Second convergence: ACP is now table stakes

kimi-cli (`kimi acp`), mistral-vibe (`vibe/acp`), hermes (`acp_adapter`), grok-build
(`xai-acp-lib`), and openclaw (`acp-core`) all speak the [Agent Client Protocol]. codex and
prime-agent have equivalents (`app-server`, daemon protocol v4). Editors drive agents over a
standard wire; not shipping it means not being usable from Zed/JetBrains.

Adopt ACP rather than inventing a protocol. clew's `src/server/` work should probably land
there too.

[Agent Client Protocol]: https://github.com/agentclientprotocol/agent-client-protocol

---

## mistral-vibe's ADRs are the best-written architecture docs in the set

Ten short ADRs, each with **Decision / Rationale / Agent Guidance / Flag To User When**.
That last section is the innovation — it tells a coding agent *when to stop and ask*.
Copy the format for cust.

The load-bearing decisions:

- **0001** — pragmatic hexagonal: core depends on models and ports; UI, FS, network,
  provider SDKs, subprocesses, protocols live at the edges. "Ports are useful when they
  protect a real boundary or make tests simpler. They are not required for every small
  helper." Plus an explicit startup-time budget: no eager imports, network calls, or broad
  FS scans on the launch path.
- **0003** — event-driven loop: typed events + streaming generators are *the* contract.
  "Do not make consumers inspect private agent-loop state to understand what happened."
  Public event IDs are monotonic; a gap is recovered by re-reading, not by patching locally.
- **0004** — typed permissioned tools: args/result/config/state all typed; permission policy
  is *part of the tool contract*. Shell permission analysis recurses into nested command
  constructs rather than pattern-matching the top-level string — this is the correct
  paranoia level, and worth stealing outright.
- **0006** — sessions: append-friendly for messages, atomic for metadata, migration-tolerant.
  Private storage format and public projection are **different contracts**; public events are
  not a persistence format. Rewind is explicitly two modes (fork vs in-place) and the
  destructive one must never be inferred from a missing option.

---

## Daemon / long-running: prime-agent is the reference implementation

Its `daemon.md` is the most rigorous process-architecture doc in the set. Structure:

- **supervisor** owns sockets, attachments, routing, agent-message delivery, worker health,
  command journals — and executes *nothing* (no providers, tools, kernels, transcripts).
- **worker** = one root session tree + its scheduler + kernels + all RLM descendants.
  Closing the TUI detaches the client; the worker keeps running.
- **catalog subprocess** owns saved-session scans, so a scan failure can't hurt live workers.

The details that matter more than the diagram:

- **Leases keyed by canonical transcript path.** Concurrent open returns `session_already_active`
  with the owner's ID. This is how you stop two processes writing one JSONL.
- **Idempotency journal keyed by `clientId + commandId`**, written *before* dispatch. Repeating a
  completed command returns the stored result; a command with no durable result is reported
  **uncertain and is not replayed**. Crash recovery appends a visible marker to the transcript and
  restores under the same session ID — it does not replay uncertain side effects.
- **Generation-aware event cursors** `{generation, sequence}`. A generation change invalidates the
  old sequence; missing replay is not fatal because the attach snapshot is the recovery baseline.
- **Scheduler ticks are claimed and advanced before delivery**, so a crash can't replay an
  uncertain prompt, and missed ticks coalesce instead of piling into a backlog.
- **Backpressure is attachment-local** — a blocked client stops receiving; nobody queues for it.
- Supervisor forwards the worker's already-serialized payload by reading only a routing header —
  it never builds a history-sized object.

Also note the honesty: "process-isolated for lifecycle and failure containment, **not**
security-sandboxed. They run with the same OS permissions as the client." Every project in
this survey says some version of this. cust should too.

---

## Compaction: three distinct designs

- **prime-agent** (best documented): trigger at `contextTokens > contextWindow - reserveTokens`
  (16384 default); walk back accumulating until `keepRecentTokens` (20k); never cut at a tool
  result; handle the **split turn** (one turn bigger than the budget → summarize history and
  turn-prefix separately, then merge); write a `CompactionEntry` with `firstKeptEntryId` and
  reload. Next compaction starts from the *previous kept boundary*, not the compaction entry.
  Fixed summary skeleton — Goal / Constraints / Progress(Done,InProgress,Blocked) / Key Decisions /
  Next Steps / Critical Context + `<read-files>` and `<modified-files>` tracked **cumulatively**
  across compactions. Conversation is serialized to `[User]:` / `[Assistant]:` / `[Tool result]:`
  lines specifically so the model doesn't try to continue it, with tool results truncated to 2000
  chars. Also has **branch summarization** for `/tree` navigation (find common ancestor, summarize
  the abandoned branch into the new one).
- **hermes micro-compaction**: after every turn, fold *one* oldest exchange into a running summary
  — amortized instead of one big stall. Off by default, and the doc is refreshingly honest about
  the cost: it rewrites already-sent history every turn, which **breaks the provider prompt-cache
  prefix**, and "for some setups that cost exceeds the benefit." Good reminder that compaction
  strategy and prompt caching are coupled.
- **codex**: a whole family — `compact.rs`, `compact_remote*.rs`, `compact_token_budget.rs`,
  `compact_model_fallback.rs`. Remote/server-side compaction is a thing at their scale.

---

## Sandboxing: grok-build has the cleanest model

Named profiles over ad-hoc flags, kernel-enforced (Landlock on Linux, Seatbelt on macOS):

| Profile | Read | Write | Child network |
|---|---|---|---|
| `off` | all | all | all |
| `workspace` | everywhere | CWD + `~/.grok/` + temps | allowed |
| `devbox` | everywhere | all top-level except `/data` | allowed |
| `read-only` | everywhere | `~/.grok/` + temps | blocked (Linux only) |
| `strict` | CWD + system paths | CWD + `~/.grok/` + temps | blocked (Linux only) |

Custom profiles in `sandbox.toml` with `extends`, `read_only`, `read_write`, and a glob `deny`
list (`**/*.pem`, `**/.env`) that is kernel-enforced for read *and* write/rename.

The subtle part worth copying: it **write-denies its own hook directories** so a compromised
agent can't install a persistent hook, refuses to start if `$GROK_HOME` is a symlink, pins parent
directories against rename, and disables nested user namespaces inside bubblewrap. It is also
explicit that macOS child-network blocking is a **no-op** — a limitation stated rather than
implied. codex splits the same job across `linux-sandbox`, `windows-sandbox-rs`, `bwrap`,
`execpolicy`, `sandboxing`, `process-hardening`.

---

## Crate-boundary lessons from the two Rust repos

1. **Provider/model is its own crate.** codex: `model-provider`, `model-provider-info`,
   `models-manager`. grok: `xai-grok-models`, `-sampler`, `-sampling-types`.
2. **Split config from config-types** (grok has both) so leaf crates depend on types without
   pulling loading logic. Cheap; prevents a dependency knot.
3. **Split tools-api from tools** (grok: `xai-grok-tools-api` vs `xai-grok-tools`). Retrofitting
   this is expensive.
4. **Shell/exec is several crates, not a function.** codex: `shell-command`, `exec`, `exec-server`,
   `exec-server-protocol`, `shell-escalation`. grok adds `ptyctl` for PTY control.
5. **Git is a subsystem, not shell-outs.** grok: `xai-gix-status` (in-process `gix`),
   `xai-hunk-tracker`, `xai-fast-worktree`. Spawning `git status` every turn is a real cost.
6. **System reminders are first-class.** grok has `system_reminder.rs` in the agent crate;
   codex has ~35 files under `core/src/context/` that are *nothing but* context injections
   (`current_time_reminder`, `token_budget_context`, `turn_aborted`, `subagent_notification`,
   `permissions_instructions`, …). This is a real subsystem, not a string concat.

codex's `AGENTS.md` also carries the best Rust style rules in the set — worth adopting verbatim:
modules under 500 LoC (new module rather than growing one past ~800), exhaustive `match` over
wildcards, no `#[async_trait]` (use RPITIT with explicit `Send`), no bool/`Option` positional
params (enums or newtypes so the callsite self-documents), inline `format!` args.

---

## Ideas from the rest

- **grok**: agents vs personas — an *agent* configures a whole session (model, tools, prompt);
  a *persona* is a behavioral overlay applied only to subagents. Two axes instead of one.
  Background tasks get `task_id` + `get_command_or_subagent_output` +
  `wait_commands_or_subagents(mode=wait_any|wait_all)` + `kill_command_or_subagent` — one
  uniform handle for both shell jobs and subagents.
- **prime-agent**: delivery modes for agent-to-agent messages — `steer` (inject into active
  work), `follow_up` (wait for the turn to end), `auto`. Receipts are `delivered` or `queued`.
  Also: **goal ≠ autonomous mode**. A goal stores the objective and its progress across turns;
  autonomous mode decides whether to inject another continuation based on gates and limits.
  Gates re-run only when the workspace changed.
- **prime-agent's continual harness**: `/refine` reviews the trajectory and applies small
  evidence-backed edits to supplemental prompts/memories/skill descriptions, with before/after
  snapshots for rollback and **the base system prompt immutable**. That immutability boundary is
  the insight — self-modification is safe exactly to the degree it is confined to a supplemental
  layer.
- **prime-agent skills**: Python-backed skills are a strict superset of markdown skills — same
  `SKILL.md` discovery, plus an installable package the model calls as `await release_audit(...)`.
  Progressive disclosure: only descriptions live in the system prompt. Supports pointing at
  `~/.claude/skills` and `~/.codex/skills` directly.
- **hermes**: seven terminal backends (local, Docker, SSH, Singularity, Modal, Daytona, Vercel
  Sandbox) behind one interface, and chat-channel delivery (Telegram/Discord/Slack) as a first-class
  surface. Its flat 200-module `agent/` package is also the clearest warning in the set about what
  happens without enforced boundaries.
- **openclaw**: model runtime *generations* — one atomic snapshot of auth template + model registry
  + catalog per config change; runs fork from it. "A failed or stale generation is never served
  alongside a newer partial generation." Directly addresses the class of bug clew's AGENT.md warns
  about with process-global `setSessionProvider`.
- **kimi-cli**: `kosong`, the LLM abstraction, is a separately published package — provider layer
  as a reusable library, not an internal folder. Also a Ctrl-X shell mode: the agent CLI doubles
  as a shell.
- **clew**: the tool-result contract `{ok, summary, data?}`; live `/models` context window
  preferred over the static table; the fallback chain's same-provider-only constraint (because
  provider switch is process-global). Its scars are worth reading before repeating them — a
  ~1866-error typecheck baseline that has been red "for a while", and a `.js` shadow-file hazard
  that Rust's module system makes impossible.

---

## Proposed crate layout

Twelve crates that respect the seams above. codex's ~120 is a Bazel-scale answer to a Bazel-scale
problem; hermes' flat package is the opposite failure.

```
crates/
  cust-code         # bin: arg parsing, composition root (binary name: `cust`)
  cust-core         # agent loop, turn state, typed events, compaction, system reminders
  cust-config       # loading, profiles, layered precedence
  cust-config-types # types only — leaf crates depend on this
  cust-provider     # providers, streaming, auth, usage/rate limits, model catalog
  cust-tools-api    # Tool trait, typed args/result, permission contract
  cust-tools        # implementations + late-bound registry
  cust-exec         # shell, PTY, sandbox profiles
  cust-codemode     # QuickJS guest + host-tool bridge
  cust-session      # transcript store, leases, rewind, resume
  cust-proto        # ACP + daemon protocol types
  cust-tui          # ratatui frontend
```

Contracts to fix before anything else, since everything hangs off them:

- **Tool result** — clew's `{ok, summary, data?}`, plus vibe's typed args/result/config/state
  and permission-as-part-of-the-contract.
- **Event stream** — typed events, monotonic IDs, gap recovered by re-read (vibe 0003).
- **Session storage** — append-friendly messages, atomic metadata, migration-tolerant; private
  format ≠ public projection (vibe 0006).
- **Provider capabilities** — `chat`/`vision`/`tool_calling`/`streaming`/`max_context` per model,
  live `/models` preferred over the static table (clew), published as an atomic generation
  (openclaw).

## Open questions

1. Code-mode only, classic tool schemas only, or both behind a config flag? (codex and MiMo ship
   *both* — a small direct tool set alongside `exec`.)
2. `rquickjs` now or `deno_core` from the start?
3. Can subagents be spawned from inside code-mode (prime-agent) or must they stay outside it
   (MiMo)? This decides whether we need a typed host-request bridge on day one.
4. ACP first, or a private protocol first with ACP adapted on top?
5. Where does clew fit — is cust a rewrite, a companion, or unrelated?
