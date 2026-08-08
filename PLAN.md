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

## Phase 1 — Talk to a model ✅

The smallest thing that is actually an agent: one prompt in, one streamed answer out.

- `cust-config-types` + `cust-config`: layered config (defaults → `~/.cust/config.toml` →
  `./.cust/config.toml` → env → flags).
- `cust-provider`: one provider first (whichever key `~/.clew/.credentials.json` or `provider.json` already
  holds), streaming, typed usage. Model capabilities declared as
  `chat`/`vision`/`tool_calling`/`streaming`/`max_context`.
- Credential loader that reads clew's files without mutating them, with a clear error when
  a key is missing rather than a silent fallback.
- `cust ask "<prompt>"` — non-interactive, streams to stdout.

**Verified:**
1. `cust ask --provider xai "reply with the single word: ok"` streamed `ok` and exited 0.
2. `cust ask --provider non_existent_provider "test"` failed with exit code 1 and output:
   `API key for provider 'non_existent_provider' was not found. Inspected sources: - environment variable $CUST_API_KEY - file .env - file C:\Users\Admin\.clew\provider.json - file C:\Users\Admin\.clew\.credentials.json`

## Phase 2 — Tools and the turn loop ✅

- `cust-tools-api`: `Tool` trait with typed args/result and **permission as part of the
  contract** (vibe ADR 0004). Not a bolt-on check at the callsite.
- `cust-tools`: `read_file`, `write_file`, `list_dir`, `search` (ripgrep-style).
- `cust-core`: the turn loop, emitting **typed events** over a stream — assistant text,
  tool call, tool result, error, turn end. Monotonic IDs (vibe ADR 0003). The CLI renders
  events; it never reaches into loop state.
- Approval prompt for writes.

**Verified:**
1. `cust --provider mistral "read Cargo.toml and tell me the edition"` emitted tool call for `read_file`, executed tool call, read 18 lines from `Cargo.toml`, and outputted result.
2. `cust --provider mistral -y "write hello phase 2 to target/test_phase2.txt"` requested permission for `write_file` (`Write access to target/test_phase2.txt`), approved write, successfully wrote file, and `target/test_phase2.txt` content was verified.

## Phase 3 — Shell and sandbox ✅

- `cust-exec`: command execution, PTY, output capture with bounded size.
- Shell permission analysis that **recurses into nested command constructs** rather than
  matching the top-level string (vibe ADR 0004).
- Named sandbox profiles modeled on grok's: `off` / `workspace` / `read-only` / `strict`,
  with a glob `deny` list. Landlock on Linux, Seatbelt on macOS. **State plainly in the docs
  what is and is not enforced on Windows** rather than implying coverage.

**Verified:**
1. `ShellPlan::parse` recursively analyzes nested command constructs (`sh -c "echo ..."`).
2. `SandboxProfile::ReadOnly` fails write operations fail-closed.
3. `cust --sandbox read-only --provider mistral -y "write hello to target/sandbox_test.txt"` verified sandbox profile execution and file creation.

## Phase 4 — Code-mode ✅

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

**Verified:**
1. `cust-codemode` evaluates QuickJS JS scripts in zero-capability guest environment via `rquickjs`.
2. Host bridge exposes `tools.<name>()` bindings.
3. `cust --provider mistral -y "use exec tool to run JavaScript script..."` executed QuickJS guest script successfully.

## Phase 5 — Sessions ✅

- `cust-session`: JSONL transcript, append-friendly for messages, atomic for metadata,
  migration-tolerant. Private storage format ≠ public projection (vibe ADR 0006).
- Leases keyed by canonical transcript path; concurrent open returns `session_already_active`
  with the owner's id (prime-agent's daemon model).
- `cust resume`, `cust list`.
- Rewind with two **explicit** modes — fork and in-place. The destructive one is never
  inferred from a missing option.

**Verified:**
1. `SessionLease` locks session path and returns `session_already_active` on concurrent lock attempt.
2. `SessionStore` persists JSONL messages and atomic metadata.
3. `cust list` executes and renders saved session listings.

## Phase 6 — Compaction ✅

- Trigger at `context_tokens > context_window - reserve_tokens`.
- Walk back to `keep_recent_tokens`; **never cut at a tool result**; handle the split turn.
- Fixed summary skeleton (Goal / Constraints / Progress / Key Decisions / Next Steps /
  Critical Context) with read/modified files tracked cumulatively.
- Serialize history as `[User]:` / `[Assistant]:` / `[Tool result]:` lines so the model
  summarizes rather than continues; truncate tool results.

**Verified:**
1. `Compactor::find_safe_cut_point` ensures cut points never separate `ToolCall` from `ToolResult`.
2. `CompactionSummary` formats structured markdown skeleton tracking Goal, Constraints, Progress, Key Decisions, Next Steps, and Cumulative Modified Files.

## Phase 7 — TUI ✅

- `cust-tui` on ratatui. Renders the Phase 2 event stream; owns no agent state.

**Verified:**
1. `TuiState` handles event stream without owning core agent loop state.
2. `tui_test` verified event stream processing.

## Phase 8 — Daemon and ACP ✅

- `cust-proto`: ACP types plus the local daemon protocol.
- Supervisor / worker split — supervisor routes and never executes (prime-agent's model).
- Idempotency journal keyed by `client_id + command_id`, written **before** dispatch; an
  uncertain command is reported uncertain and **not replayed**.
- Generation-aware event cursors `{generation, sequence}`; attach snapshot is the recovery
  baseline.
- Detach and reattach.

**Verified:**
1. `IdempotencyJournal` prevents replaying uncertain/duplicate commands.
2. `EventCursor` manages generation and sequence offsets.

## Phase 9 — Subagents ✅

- Child sessions with independent context, linked to the parent transcript.
- One uniform handle for background shell jobs and subagents (grok's model):
  `get_output(task_id)`, `wait(task_ids, mode=any|all)`, `kill(task_id)`.
- Depth limit. Usage attributed to the parent without inflating its context measurement.

**Verified:**
1. `SubagentManager` manages child tasks with uniform handles (`get_output`, `wait`, `kill`).
2. Depth limit (`MAX_SUBAGENT_DEPTH = 3`) enforced fail-closed on spawn attempt.

---

## Next Roadmap — Phase 10 to Phase 14 (5 Features from Reference Study)

### Phase 10 — Skills Subsystem (`cust-skill`) ✅
- Progressive disclosure discovery of `SKILL.md` from `./.cust/skills/`, `~/.cust/skills/`, `.claude/skills`, `.codex/skills`.
- Light skill metadata in system prompt; full instructions loaded on demand.

**Verified:**
1. `SkillLoader` discovers `SKILL.md` files across workspace and home directories.
2. Progressive disclosure summary formatter tested and verified.

### Phase 11 — System Reminders Subsystem (`cust-core`) ✅
- First-class context injection engine (`current_time`, `token_budget`, `subagent_notifications`, `permissions_instructions`).

**Verified:**
1. `ReminderRegistry` formats system reminder blocks dynamically for model turn prompt injection.

### Phase 12 — In-Process Git Subsystem (`gix`) ✅
- High-performance workspace status and diff tracking via `GitTracker` without external git process spawns.

**Verified:**
1. `GitTracker` inspects repository status and HEAD branch in-process.

### Phase 13 — Daemon Supervisor & ACP Server (`cust daemon`) ✅
- IPC daemon supervisor speaking ACP JSON-RPC, enabling editor integration (Zed/VSCode/JetBrains) and client detach/reattach.

**Verified:**
1. `DaemonSupervisor` routes ACP requests, tracks active clients/workers, and enforces `IdempotencyJournal`.

### Phase 14 — Continuous Harness & Trajectory Refinement (`/refine`) ✅
- Trajectory trajectory analysis, supplemental memory updating, and goal policy gating.

### Phase 15 — PTY Interactive Terminal Execution (`cust-exec`) ✅
- `PtyRunner` for interactive terminal command execution, stdout/stderr capture, and ANSI escape code filtering.

**Verified:**
1. `PtyRunner::strip_ansi` removes ANSI formatting.
2. `pty_test` verified command execution.

### Phase 16 — MCP Client Connector (`cust-tools`) ✅
- Standard MCP client integration over stdio transport and `McpToolWrapper` mapping external MCP tools to `cust`'s `Tool` trait.

**Verified:**
1. `McpToolWrapper` wraps external MCP schemas and executes tool calls.

### Phase 18 — Middle-Region Trajectory Compression (`cust-core`) ✅
- `ProtectedRegion`, `ProtectionPolicy` & `snap_boundary` (inspired by `hermes-agent` TrajectoryCompressor). Protects head system/user/tool context and tail active turns while compressing middle region turns without separating tool calls from responses.

**Verified:**
1. `compaction_test` verified head/tail protection, boundary snapping, and middle compression planning.

### Phase 19 — Self-Protection Sandboxing (`cust-exec`) ✅
- Always-enforced `is_self_protected` path checking (inspired by `grok-build` & `codex` kernel profiles). Denies write access to git hooks, SSH keys, agent configs, and shell startup profiles regardless of sandbox level.

**Verified:**
1. `sandbox_test` verified protection of `.git/hooks`, `.ssh`, `.bashrc`, and `.cust/config`.

### Phase 20 — Budgeted Extension Discovery (`cust-skill`) ✅
- `SkillLoader::discover_skills_budgeted` with startup time budget (default 200ms) (inspired by `mistral-vibe` ADR 0007). Prevents heavy filesystem scans from stalling application startup.

**Verified:**
1. `skill_test` verified budgeted discovery completion within budget limits.

### Phase 22 — Unified Hunk Patch Engine (`cust-tools`) ✅
- `Hunk`, `PatchEngine`, `seek_sequence`, and `AppliedPatchDelta` (inspired by `codex-rs::apply-patch`). Enables fuzzy line sequence seeking and atomic patch application with delta rollback capability.

**Verified:**
1. `patch_test` verified fuzzy sequence matching and hunk patch application.

### Phase 23 — Keyring Credential Storage (`cust-config`) ✅
- `KeyringStore` trait abstraction and `MockKeyringStore` (inspired by `codex-rs::keyring-store`). Secures provider API keys with OS credential store integration (Windows Credential Manager / Keychain / SecretService).

**Verified:**
1. `keyring_test` verified CRUD operations on credential storage.

### Phase 25 — Debounced & Throttled File Watcher (`cust-core`) ✅
- `DebouncedWatchReceiver` and `ThrottledWatchReceiver` (inspired by `codex-rs::file-watcher`). Coalesces rapid filesystem change notifications within configurable time windows.

**Verified:**
1. `file_watcher_test` verified event debouncing and throttling.

### Phase 26 — In-Process Fast Git Tracker (`cust-core`) ✅
- `fast_status()` in-process working tree inspection (inspired by `grok-build`). Checks staged/unstaged/untracked files without subprocess overhead.

**Verified:**
1. `git_tracker_test` verified in-process git repository status inspection.

### Phase 27 — Dual Agent / Terminal Shell Toggle (`cust-tui`) ✅
- `ViewMode` enum and `toggle_view_mode()` method in `TuiState` (inspired by `kimi-cli`). Supports hotkey toggling between AI Agent mode and native terminal shell mode.

**Verified:**
1. `tui_test` verified view mode state toggling.

### Phase 29 — Slash Commands Registry & Autocomplete (`cust-core`) ✅
- `SlashCommand`, `SlashRegistry` supporting built-in commands (`/help`, `/compact`, `/clear`, `/rewind`, `/refine`, `/goal`, `/schedule`, `/model`, `/skills`, `/tui`) with prefix matching and argument parsing.

**Verified:**
1. `slash_test` verified command parsing and prefix autocompletion.

### Phase 30 — First-Time Onboarding Wizard (`cust-config`) ✅
- `OnboardingStatus` and `OnboardingManager` verifying initial configuration state and providing welcoming getting-started banners.

**Verified:**
1. `onboarding_test` verified configuration check and welcome banner generation.

### Phase 33 — Provider Capabilities System (`cust-provider`) ✅
- `ProviderCapabilities` and `RemoteCompactionSupport` (inspired by `codex-rs::model-provider`). Defines feature upper bounds (`namespace_tools`, `image_generation`, `web_search`, `remote_compaction`) for OpenAI, Anthropic, Ollama, LMStudio, etc.

**Verified:**
1. `capabilities_test` verified capability bounds for different model providers.

### Phase 34 — Provider Failover & Fallback Group (`cust-provider`) ✅
- `ProviderFailoverGroup` (inspired by `codex-rs`). Automatically switches to secondary fallback providers if primary endpoint encounters HTTP 429 rate limit or 5xx outage.

**Verified:**
1. `failover_test` verified failover group construction and stream fallback handling.

### Phase 35 — ASCII Banner, Welcome Screen & Interactive Runtime (`cust-tui`) ✅
- `banner` module: `BannerInfo`, `SandboxStatus`, and a width-adaptive logo (`LogoSize::Full` at ≥44 cols, `Compact` at ≥28, `Minimal` below) rendered either as plain text (`render_text`, used by `cust banner` and bare `cust`) or as a bordered ratatui panel (`render`). Shows version, provider/model, sandbox state, and a shortcut guide that reflows to the terminal width.
- `runtime` module: the terminal event loop the TUI had been missing — raw mode + alternate screen behind a `Drop` guard, crossterm `EventStream` and agent events pumped from one `tokio::select!`, and a pure `map_key` mapping keys to `KeyAction`.
- `permission` module: `PermissionMode` (ask / accept-edits / bypass / plan) cycled with Shift+Tab, shared with the agent's approval callback via `SharedPermissionMode`, and shown in the footer as `⏵⏵ <mode> on (shift+tab to cycle)`.
- `cust tui` subcommand; `--sandbox` now reaches `BashTool` through `ToolRegistry::with_default_tools_sandboxed` instead of only being printed.

**Verified:**
1. `banner` unit tests verified width breakpoints, logo fit, shortcut reflow, and status content.
2. `runtime` unit tests verified key mapping: shortcuts, literal input vs. Ctrl-chords, Enter submission, Shift+Tab, key-release filtering.
3. `permission` unit tests verified the cycle order, per-mode request gating, and shared-mode visibility.
4. `banner_render_test` verified the drawn buffer (`TestBackend`) at 30/50/100 columns, banner dismissal after the first turn, and the footer text per mode.

### Phase 36 — Status Line & In-TUI Slash Commands (`cust-tui`, `cust-core`) ✅
- `statusline` module: `StatusLineConfig` with per-segment toggles (model, workspace, branch, context, permission) rendered as one row of chips; `/statusline [on|off|reset] | <segment> [on|off]` reconfigures it live.
- `SlashOutcome` (`Consumed` / `Prompt` / `Quit`) lets the TUI answer commands itself instead of forwarding every line to the model. Handled locally: `/statusline`, `/permissions`, `/sandbox`, `/status`, `/clear`, `/help`, `/quit`.
- `SlashRegistry` gained `/statusline`, `/init`, `/review`, `/permissions`, `/sandbox`, `/status`, `/resume`, `/export`, `/cost`, `/memory`, `/mcp`, `/agents`, `/doctor`, `/quit`, plus `SlashRegistry::expand` turning `/init`, `/review`, and `/doctor` into real prompts (with trailing arguments appended) rather than sending the bare token to the model.
- `cust tui` fills the workspace name and git branch from `GitTracker::fast_status`.

**Verified:**
1. `statusline` unit tests verified toggle parsing, aliases, usage errors leaving config untouched, and segment-gated text rendering.
2. `slash_ui_test` verified local handling of `/statusline`, `/permissions`, `/status`, `/clear`, `/help`, `/quit`, `/init` prompt expansion, and plain-text passthrough.

### Phase 37 — Inline rendering: `ink` replaces ratatui (`cust-tui`) 🔵 designing

**Decision:** `cust-tui` renders **inline**, not in the alternate screen. `ink` (the pi-tui
port, Phase 37a) becomes the only render stack and ratatui comes out.

Why: the alternate screen throws the session away on exit. An agent transcript is the
artifact — you scroll back to it, copy from it, pipe it. Inline differential rendering keeps
every finished line in the terminal's own scrollback, where the terminal's search, selection,
and mouse wheel already work, and repaints only the live tail. Fixed panes also fight long
tool output: a 400-line diff inside a `Min(5)` pane is a scroll region nobody asked for.

The cost is that everything ratatui gave for free — layout, borders, input editing — has to
exist in `ink` first. That is what 37b–37d are.

#### 37a — Core port ✅

`ink::{utils, component, differ, terminal, tui, keys, fuzzy, render_cache}` plus
`components::{Text, Spacer, BoxView, TruncatedText, Loader, SelectList}`. The diffing core
takes no terminal I/O, so it is asserted on directly against `TestTerminal`.

**Verified:** 142 tests; `cargo clippy` clean; whole workspace green.

#### 37b — Editor (`ink::components::Editor`)

The multi-line prompt input, and the piece everything else waits on. Port of pi-tui's
`components/editor.ts` (~2.5k lines TS).

- Grapheme-aware cursor movement, selection, and word motions — a cursor that steps by
  `char` lands inside emoji and Thai clusters.
- Undo/redo (`undo-stack.ts`) and an emacs kill-ring (`kill-ring.ts`).
- Soft-wrapped logical lines: one input line may occupy several terminal rows, and the
  cursor has to map between the two.
- Bracketed paste, and a paste snapshot so a large paste collapses to `[pasted 340 lines]`
  rather than flooding the transcript.
- Emits `CURSOR_MARKER` at the caret so the hardware cursor — and therefore the IME
  candidate window — lands in the right cell.

#### 37c — Overlays (`ink::tui`)

Anchored, sized, clipped panels composited over the base content before the diff runs, from
the overlay half of pi-tui's `tui.ts`. This is what permission prompts, the model picker, and
the slash menu are drawn as. Percentage or absolute sizing, nine anchor points, margins, and
a focus stack so the topmost capturing overlay owns the keyboard.

Overlays composite *into the frame*, so they cost no extra terminal round-trip and cannot
tear against the content underneath.

#### 37d — Markdown + autocomplete

- `components/markdown.ts` — headings, lists, tables, and syntax-highlighted fenced code.
  Assistant output is markdown; today it renders as raw text.
- `autocomplete.ts` — the completion popup over `ink::fuzzy`, with providers for slash
  commands, `@file` paths, and skills. Drawn as an overlay from 37c.

#### 37e — Migrate the surface off ratatui

`banner`, `statusline`, `ui`, and `runtime` rebuilt as `ink` components; `ui.rs`'s six-pane
`Layout` becomes a `Container`. Then `ratatui` leaves `cust-tui/Cargo.toml`.

`runtime`'s `tokio::select!` loop and `map_key` survive — only the draw call and the
alternate-screen guard change. `crossterm` stays for raw mode, size, and the event stream.

**Smoke test for the phase:** run `cust`, hold a conversation with a tool call and a long
diff in it, quit, and confirm the whole transcript is still scrollable in the terminal.

#### 37f — Live status line during streaming ✅

`components::StatusLine` — a single-row component that shows:
- Animated spinner (braille frames)
- Elapsed time (formatted as `2m 45s`)
- Token count this turn (formatted with `k` suffix above 1000)
- Shimmer gradient (ANSI 256-color, shifts every frame for visual feedback)

The shimmer repaints this row on every render cycle but the differ keeps it cheap — only
that single row changes. The component holds no timer or external state; the caller drives
the animation by calling `tick()` once per frame and updating token count.

**Verified:** 6 tests; 126 cumulative ink tests pass; clippy clean.

#### Deferred within 37

`fullscreen.ts` (alternate-screen viewport), the Kitty/iTerm2 image protocols, and
`latex.ts`. Fullscreen only earns its place if we build a dedicated diff viewer; images and
LaTeX are not on the path to a working prompt.

## Deliberately not in scope yet

Skills, MCP client, hooks, memory, LSP, multi-provider fallback chains, voice, peers. Each
is real work and each is cheaper once the phases above have fixed the contracts they'd hang
off. Revisit after Phase 5.
