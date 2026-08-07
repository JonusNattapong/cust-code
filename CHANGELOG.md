# Changelog

All notable changes to this project are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Phase 34: Provider Failover & Fallback Group**
  - Added `ProviderFailoverGroup` in `cust-provider` for automatic fallback switching on HTTP 429/5xx errors.
- **Phase 33: Provider Capabilities System**
  - Added `ProviderCapabilities` & `RemoteCompactionSupport` in `cust-provider` for capability upper bounds checking.
- **Phase 30: First-Time Onboarding Wizard**
  - Added `OnboardingManager` & `OnboardingStatus` in `cust-config` for initial setup validation.
- **Phase 29: Slash Commands Registry & Autocomplete**
  - Added `SlashRegistry` in `cust-core` supporting built-in slash commands (`/help`, `/compact`, `/clear`, `/rewind`, `/refine`, `/goal`, `/schedule`, `/model`, `/skills`, `/tui`).
- **Phase 24: Custbox Container Isolation Config**
  - Added `CustboxConfig` & `CustboxMount` in `cust-exec` for containerized path isolation and egress rules (inspired by `openclaw`).
- **Phase 23: Keyring Credential Storage**
  - Added `KeyringStore` trait & `MockKeyringStore` in `cust-config` for securing API keys in OS credential store (inspired by `codex-rs`).
- **Phase 22: Unified Hunk Patch Engine**
  - Added `Hunk`, `PatchEngine`, `seek_sequence`, & `AppliedPatchDelta` in `cust-tools` for atomic patch application (inspired by `codex-rs`).
- **Phase 20: Budgeted Extension Discovery**
  - Added `discover_skills_budgeted` in `cust-skill` enforcing 200ms max discovery budget (inspired by `mistral-vibe` ADR 0007).
- **Phase 19: Self-Protection Sandboxing**
  - Added `is_self_protected` path enforcement in `cust-exec` blocking writes to `.git/hooks`, `.ssh`, agent configs, and shell profiles (inspired by `grok-build` / `codex`).
- **Phase 18: Middle-Region Trajectory Compression**
  - Added `ProtectedRegion`, `ProtectionPolicy`, and `snap_boundary` in `cust-core` (inspired by `hermes-agent` TrajectoryCompressor).
- **Phase 16: MCP Client Connector**
  - Added `McpToolWrapper` in `cust-tools` mapping external MCP tool definitions over stdio JSON-RPC to native `cust` tools.
- **Phase 15: PTY Interactive Terminal Execution**
  - Added `PtyRunner` in `cust-exec` for interactive terminal command execution and ANSI escape code filtering.
- **Prime Agent Innovations**
  - Added Direct Agent-to-Agent Messaging (`AgentMessage`, `DeliveryMode` `steer` vs `follow_up`) in `cust-core`.
  - Added Goal Policy Tracker (`GoalTracker`, `GoalStatus`) & Heartbeat Scheduler (`HeartbeatScheduler`) in `cust-core`.
  - Added Executable Script Entrypoint detection (`script_path`) to `Skill` in `cust-skill`.
- **Phase 14: Continuous Harness & Trajectory Refinement (`/refine`)**
  - Added `RefineEngine` in `cust-core` for trajectory analysis, versioned supplemental memory updating, and goal policy gating.
- **Phase 13: Daemon Supervisor & ACP Server (`cust daemon`)**
  - Added `DaemonSupervisor` in `cust-proto` managing IPC client/worker sessions with ACP JSON-RPC routing and idempotency journaling.
- **Phase 12: In-Process Git Subsystem (`gix`)**
  - Added `GitTracker` in `cust-core` for in-process working tree inspection and status tracking.
- **Phase 11: System Reminders Subsystem**
  - Added `ReminderRegistry` and `ReminderKind` in `cust-core` for dynamic turn prompt context injection (`current_time`, `token_budget`, `subagent_notification`).
- **Phase 10: Skills Subsystem (`cust-skill`)**
  - Crate `cust-skill`: `SkillLoader` with progressive disclosure discovery from `./.cust/skills/`, `~/.cust/skills/`, `.claude/skills`, `.codex/skills`.
- **Phase 9: Subagents**
  - Added `SubagentManager` in `cust-core` for managing subagents with uniform task handles (`get_output`, `wait`, `kill`) and depth limit enforcement.
- **Phase 8: Daemon and ACP**
  - Crate `cust-proto`: ACP protocol types (`AcpRequest`, `AcpResponse`), `IdempotencyJournal` (`client_id + command_id`), and `EventCursor`.
- **Phase 7: TUI**
  - Crate `cust-tui`: Terminal UI renderer (`ratatui`, `crossterm`) consuming event stream without owning agent loop state.
- **Phase 6: Compaction**
  - Added `compaction` module in `cust-core` (`Compactor`, `CompactionSummary`).
  - Implemented safe cut-point calculation avoiding split turns, rough token estimation, and structured summary skeleton generation.
- **Phase 5: Sessions**
  - Crate `cust-session`: JSONL message persistence, atomic metadata (`SessionStore`), session process leases (`SessionLease`), `session_already_active` lock error, and explicit rewind modes (`Fork`, `InPlace`).
  - Added CLI `cust list` command to display saved sessions.
- **Phase 4: Code-mode**
  - Crate `cust-codemode`: Embedded QuickJS guest engine (`rquickjs`) with zero-capability environment and typed host bridge (`tools.<name>()`).
  - Added `exec` tool in `cust-tools` invoking QuickJS code mode interpreter.
- **Phase 3: Shell and sandbox**
  - Crate `cust-exec`: Process execution with bounded output capture, recursive command parser (`ShellPlan`), and sandbox profiles (`Off`, `Workspace`, `ReadOnly`, `Strict`).
  - Added `bash` tool in `cust-tools` using `cust-exec`.
  - Added `--sandbox <profile>` CLI option in `cust-code`.
- **Phase 2: Tools and the turn loop**
  - Crate `cust-tools-api`: `Tool` trait, `ToolResult`, `ToolError`, `Availability`, and `PermissionRequest` declared as part of tool contract (vibe ADR 0004).
  - Crate `cust-tools`: `read_file`, `write_file`, `list_dir`, `search` tools and `ToolRegistry`.
  - Crate `cust-core`: `AgentLoop` and monotonic `Event` stream (`TurnStarted`, `AssistantDelta`, `ToolCall`, `ApprovalRequested`, `ToolResult`, `TurnEnded`).
  - Interactive approval prompts in CLI for filesystem write operations (`WritePath`).
- **Phase 1: Talk to a model**
  - Workspace crates: `cust-config-types`, `cust-config`, `cust-provider`, `cust-code`.
  - Layered configuration loader: Defaults -> `~/.cust/config.toml` -> `./.cust/config.toml` -> env -> CLI flags.
  - Read-only clew credential loader (`~/.clew/provider.json`, `~/.clew/.credentials.json`, `.env`) with explicit error messages listing searched paths when keys are missing.
  - Streaming LLM provider integration for OpenAI-compatible APIs (OpenAI, xAI, Mistral, OpenRouter, Custom) and Anthropic API.
  - CLI command `cust ask "<prompt>"` that streams LLM responses directly to `stdout`.
- Cargo scaffold: crate `cust-code`, binary `cust`, with `help` and `version` commands.
- Integration tests in `tests/cli.rs` that run the real binary, so PLAN.md's smoke checks
  are enforced rather than remembered.
- `.knowledge/` â€” the long-form study of nine coding agents (codex, grok-build, kimi-cli,
  mistral-vibe, hermes-agent, MiMo-Code, openclaw, prime-agent, clew-code), split by
  subsystem: code mode, daemon/sessions, compaction, sandboxing, tools/events, subagents,
  skills/extensions, boundaries, providers, protocols.
- `DESIGN-NOTES.md` â€” condensed version of the above.
- `ARCHITECTURE.md` â€” the synthesized design: core contracts (`Tool`, event stream, session,
  permission), crate map with dependency rules, and a table of what each subsystem takes from
  which surveyed project. `cust` is a new design rather than a clew rewrite.
- `PLAN.md` â€” phased build order with a smoke test per phase.
- `.doplan/` â€” the working task list, one file per phase.
- `.learning/` â€” lessons and errors from building this project.
- `.memory/` â€” durable facts, decisions with their reasoning, and working preferences.
