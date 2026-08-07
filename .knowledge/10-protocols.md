# Protocols and delivery surfaces

## ACP is table stakes

The [Agent Client Protocol](https://github.com/agentclientprotocol/agent-client-protocol)
appears in five of the nine projects, and the other two large ones have close equivalents.

| Project | Surface |
|---|---|
| kimi-cli | `kimi acp` — configured in Zed and JetBrains as an agent server |
| mistral-vibe | `vibe/acp`, `vibe-acp.spec` frozen binary |
| hermes-agent | `acp_adapter/` |
| grok-build | `xai-acp-lib` |
| openclaw | `packages/acp-core` |
| codex | `app-server*` (7 crates) — equivalent, not ACP |
| prime-agent | daemon protocol v4 + RPC/JSON modes — equivalent, not ACP |

kimi's config is the whole integration:

```json
{ "agent_servers": { "Kimi CLI": { "type": "custom", "command": "kimi", "args": ["acp"] } } }
```

An agent that does not speak ACP is not usable from Zed or JetBrains. Adopt rather than
invent.

## Delivery surfaces are a named concept (vibe ADR 0002)

`vibe/core` supplies the private engine — agent loop, tools, LLM backends, config, sessions,
skills, hooks, telemetry, domain models. `vibe/app_server` is the composition root that owns
those objects. Surfaces adapt it:

- `vibe/cli` — Textual app, terminal UX, widgets, slash-command presentation, voice UI;
- `vibe/acp` — thin protocol translation, over the app-server client API;
- `vibe/setup` — first-run onboarding;
- programmatic entry points — the client API without Textual or ACP internals.

Two facades: **`AppServerHost`** (passive, pre-session: list, read, delete, trust, open) and
**`AppServerSession`** (attached: turns, callbacks, resources, live events).

The rules:

- UI behavior in `cli`, protocol translation in `acp`, **never in core**.
- Core events and models stay **surface-neutral**; surfaces render or translate.
- **"Do not create a second core adapter for ACP or `-p` mode."**
- A delivery surface **must not consume `AgentLoop.act()` directly** (ADR 0003).

Flag-to-user: core needing to import Textual or ACP schema objects; a protocol-specific
workaround in a core model; programmatic mode depending on interactive terminal assumptions.

The single most valuable structural idea here: **one engine, N surfaces, and the surfaces get
the same public API a third party would get.**

## prime-agent's daemon protocol v4

A concrete feature list for a local agent protocol:

- versioned command envelopes with stable client and command IDs;
- capability negotiation and per-command compatibility metadata;
- generation-aware event cursors `{generation, sequence}`;
- reconnect with a stable identity and a resume cursor;
- attach acknowledgment plus coherent snapshots;
- begin/chunk/end snapshot streaming, 512 KiB target chunk, file-backed above 4 MiB;
- resident and client-owned worker lifecycle commands;
- daemon-side headless completion, session-header, bash, and retry operations;
- structured errors for recoverable cases — `session_already_active`, uncertain mutation.

Versioning discipline worth copying:

> Protocol version and schema revision are **independent**. A compatible addition can be
> capability-gated or require a schema revision; an incompatible wire change requires a
> protocol bump.

And a leakage rule: "JSON and RPC client modes do not expose daemon greetings, envelopes,
snapshot records, lifecycle events, or connection metadata." Internal transport concepts must
not leak into public output contracts.

Old protocol versions are retained **only** for the one-release update handoff that prepares
and stops an older daemon — and "a busy older daemon that cannot produce a recovery manifest
is left running." Update refuses to be destructive.

## Coordinated updates (two-phase)

1. Resident workers create non-destructive checkpoints in parallel.
2. The supervisor validates and atomically persists the aggregate manifest.
3. **Only after every prepare succeeds** does it commit and stop workers.

Failure at prepare or validation releases the prepared workers and all roots keep running.
Two-phase commit for "update the agent while sessions are live."

## Headless modes

Standard set across the field: interactive TUI, print/one-shot, piped stdin, JSON output,
RPC (LF-delimited JSONL, prompts until EOF), plus `--no-session` for in-memory runs.

prime-agent's insight is that these should be **the same runtime with a different lifecycle**
(client-owned workers), not a separate code path — while keeping their public I/O contracts
stable. "Direct SDK calls to print and RPC modes remain in-process so embedders can pass
non-serializable extension factories."

## Beyond the terminal

- **hermes** — Telegram, Discord, Slack, WhatsApp, Signal, and CLI from a single gateway
  process, with voice-memo transcription and cross-platform conversation continuity.
- **openclaw** — an explicit Gateway connecting models, tools, messaging channels, and
  companion apps for a single operator; `gateway-protocol` and `gateway-client` packages.
- **MiMo** — `packages/slack`, `packages/desktop`, `packages/web`, `packages/extensions`.
- **kimi** — a VS Code extension, and Zsh integration.

Chat channels as a first-class surface is a real trend, and it only works if the engine and
protocol boundary were drawn correctly in the first place.

## Runtime selection (openclaw)

Multiple agent runtimes behind one CLI, selected by model/provider-scoped config, with
plugin-registered harnesses. The care in the rules is instructive:

> OpenAI may select `codex` implicitly **only** for an exact official HTTPS Platform
> Responses or ChatGPT Responses route with no authored request override. Completions
> adapters, custom endpoints, and routes with authored request behavior stay on `openclaw`;
> plaintext official HTTP endpoints are rejected. **A provider or model prefix alone never
> selects a harness.**

Implicit behavior switching needs exact matching rules, or it becomes unpredictable.

## For cust

- ACP from Phase 8, not invented.
- Engine / composition root / surfaces split from the start (vibe 0002) — cheap now,
  expensive later.
- Headless modes = same runtime, client-owned lifecycle.
- Protocol version and schema revision independent; structured errors for recoverable cases;
  no transport concepts in public output.
