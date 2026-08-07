# Knowledge base

What was learned from reading nine coding agents before writing `cust-code`. This is the
long form; [`../DESIGN-NOTES.md`](../DESIGN-NOTES.md) is the condensed version and
[`../PLAN.md`](../PLAN.md) is what we do about it.

Read in full for each project: architecture docs, ADRs, user guides, and crate/package
layouts. Implementation internals were read only where quoted. Sources live in
`clew-code/.reference/`.

## The nine

| Project | Org | Lang | Shape | Read |
|---|---|---|---|---|
| codex | OpenAI | Rust + TS | ~120 crates, Bazel + Cargo | AGENTS.md, docs/, crate tree, code-mode protocol |
| grok-build | xAI | Rust | ~60 crates | README, 24-part user guide, crate tree |
| kimi-cli | Moonshot | Python | `src/kimi_cli` + `kosong` | README, docs tree |
| mistral-vibe | Mistral | Python | `vibe/core` + surfaces | all 10 ADRs |
| hermes-agent | Nous | Python + TS | flat `agent/` (~200 modules) | README, micro-compaction doc |
| MiMo-Code | Xiaomi | TS (Bun) | 17 packages | microkernel-runtime architecture doc |
| openclaw | community | TS | 22 packages + gateway | agent-runtime-architecture |
| prime-agent | Prime Intellect | TS | 4 packages | architecture, rlm, rlm-runtime, daemon, compaction, skills, long-running |
| clew-code | us | TS (Bun) | one `src/` | AGENT.md |

## Files

| File | Topic |
|---|---|
| [01-code-mode.md](01-code-mode.md) | **The big one.** Four teams independently replaced tool schemas with a sandboxed interpreter |
| [02-daemon-and-sessions.md](02-daemon-and-sessions.md) | Supervisor/worker split, leases, idempotency, crash recovery, session storage |
| [03-compaction.md](03-compaction.md) | Three compaction designs and the prompt-cache tradeoff |
| [04-sandboxing.md](04-sandboxing.md) | Kernel-enforced profiles, self-protection, honest limits |
| [05-tools-and-events.md](05-tools-and-events.md) | Tool contracts, permission-in-the-contract, typed event streams |
| [06-subagents.md](06-subagents.md) | Delegation, agent-to-agent messaging, goals vs autonomous mode |
| [07-skills-and-extensions.md](07-skills-and-extensions.md) | Skills, hooks, MCP, plugin surfaces, self-improvement |
| [08-boundaries.md](08-boundaries.md) | Crate/module boundaries, and the two failure modes at either extreme |
| [09-providers.md](09-providers.md) | Model catalogs, capabilities, generations, fallback |
| [10-protocols.md](10-protocols.md) | ACP and the daemon wire formats |

## The five things that would change how you build an agent

1. **Give the model an interpreter, not a tool list.** Four teams landed here
   independently. A five-step pipeline costs five round trips as tool calls and one as a
   script. → [01](01-code-mode.md)
2. **The sandbox isolates code, not effects.** The interpreter gets zero capabilities;
   every real side effect goes back out to a host tool with normal permission checks, via a
   late-bound registry so hidden tools cannot reappear. → [01](01-code-mode.md)
3. **Permission belongs in the tool contract**, and shell analysis must recurse into
   nested command constructs rather than matching the top-level string. → [05](05-tools-and-events.md)
4. **A command with no durable result is uncertain and must not be replayed.** Journal
   before dispatch, key by `client_id + command_id`, append a visible recovery marker.
   → [02](02-daemon-and-sessions.md)
5. **Compaction and prompt caching are coupled.** Rewriting already-sent history breaks the
   provider cache prefix; the cost can exceed the benefit. → [03](03-compaction.md)
