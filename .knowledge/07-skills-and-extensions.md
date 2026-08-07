# Skills, hooks, extensions, self-improvement

## Skills have standardized

Most of the field now implements the [agentskills.io](https://agentskills.io/specification)
`SKILL.md` format: prime-agent (leniently, warning on violations), hermes, mistral-vibe,
grok, clew. Cross-loading is normal — prime-agent explicitly documents pointing at other
harnesses' directories:

```json
{ "skills": ["~/.claude/skills", "~/.codex/skills"] }
```

Interoperating is cheaper than inventing a format, and it means a new agent starts with a
usable skill ecosystem on day one.

## Progressive disclosure is the mechanism

1. At startup, scan skill locations and extract **name, description, type, location only**.
2. Put that metadata in the system prompt (XML per the spec).
3. When a task matches, the agent loads the full `SKILL.md` on demand.
4. It then follows the instructions, using relative paths for scripts and assets.

Only descriptions are always in context. prime-agent notes the practical failure —
"models don't always do this" — and provides `/skill:name` to force a load. Worth planning
for: discovery that depends on the model choosing to look is discovery that sometimes
doesn't happen.

`disable-model-invocation: true` hides a skill from the startup list while keeping it
explicitly invocable.

## Python-backed skills — a strict superset

prime-agent's extension of the format. Same `SKILL.md` for discovery and instructions, plus a
Python package installed into the kernel environment and exposed by import name:

```python
report = await release_audit(repository=".", target_version="0.4.0")
```

So a skill can provide guidance, scripts, references, dependencies, **typed callables**, and
optional shell commands — and can itself call `rlm(...)` to delegate recursively.

This only works because of code mode. In a schema-based agent a skill is documentation; in a
code-mode agent a skill is a **library**. That is a materially different capability and it
is the strongest argument for code mode after the round-trip saving.

Discovery precedence (prime-agent, worth copying wholesale): CLI `--skill` > settings >
package manifests > project dirs > global dirs > built-in. Built-ins have the **lowest**
precedence, so a user skill of the same name overrides one we ship.

## Hooks

Every project has them: grok (`xai-grok-hooks`, `xai-hooks-plugins-types`), codex (`hooks`,
`core/src/hook_runtime.rs`, `tools/hook_names.rs`), clew (`services/plugins/` — pre/post
tool/bash/prompt/edit), kimi (`hooks/`), vibe (`agent_loop_hooks.py`).

The cheapest extensibility point that exists — and, as [04-sandboxing](04-sandboxing.md)
shows, a persistence vector that needs kernel-level write protection on its own config.

vibe's constraint: "keep hooks and external processes bounded by timeouts and typed
invocation/response models."

## Extension mechanisms as a closed set (vibe ADR 0007)

> Vibe extends through explicit mechanisms: agents, subagents, skills, hooks, MCP servers,
> connectors, custom tools, and config layers.

Guidance:

- **Prefer an existing extension mechanism before adding a new one.** The flag-to-user
  condition is literally "a feature adds a new extension path instead of using skills,
  agents, hooks, MCP, connectors, tools, or config."
- Keep discovery deterministic and cheap; defer expensive integration until needed.
- **Reserve built-in names**; do not silently override built-ins with local extensions.
- Report configuration issues **without crashing the whole app** when it is safe to continue.
- Clients get typed resource views, never registries or managers.

## Resource manifests (openclaw)

Packages declare what they contribute in `package.json`:

```json
{ "openclaw": { "extensions": ["extensions/index.ts"], "skills": ["skills/*.md"],
                "prompts": ["prompts/*.md"], "themes": ["themes/*.json"] } }
```

with fallback to conventional directory names. Declarative beats convention-only for
predictable discovery, and it lets ordinary package managers distribute agent capabilities.

## Self-improvement — three approaches

### prime-agent: the continual harness

The most carefully bounded design in the survey. `rlm.harness` is a persisted state ledger
for prompt notes, memories, reusable skill descriptions, sub-agent specifications, and
refinement events — "**not a second execution engine**."

- Session-local state: `<session-artifacts>/harness/harness_state.json`.
- Explicitly global entries: `~/.prime/agent/harness/`.
- The Python store reloads after external modification so host-side `/refine` writes and
  kernel writes do not clobber each other.
- `/refine` runs a dedicated review over the current trajectory and applies small
  create/update/delete edits, with recorded before/after snapshots for rollback.
- **The base system prompt remains immutable; refinements are supplemental state.**

That last line is the insight: self-modification is safe exactly to the degree it is confined
to a supplemental layer with rollback. An agent that can rewrite its own base prompt has no
fixed point to recover to.

The docs also draw the boundary against skills: `/refine` "does not replace packaging and
reviewing new executable skills." Lessons and capabilities are different artifacts.

### hermes: the learning loop

Marketed as the differentiator — creates skills from experience, improves them during use,
nudges itself to persist knowledge, FTS5 search over its own past sessions with LLM
summarization for cross-session recall, and Honcho dialectic user modeling. Module names
show the shape: `curator.py`, `learn_prompt.py`, `learning_graph.py`,
`learning_mutations.py`, `insights.py`, `verification_evidence.py`.

### clew

`longTermMemory/`, `autoDream/`, `extractMemories/`, SQLite memory store, FTS5 session
search, `checkpoint/` and `goal/` for progress snapshots and verification.

## MCP is universal

Every project ships an MCP client. Notable variations:

- **codex** — `codex-mcp`, `rmcp-client`, `mcp-server` (it is also a server), plus
  `mcp_tool_exposure.rs`, `mcp_skill_dependencies.rs`, `mcp_prewarm.rs`, `mcp_refresh.rs`.
  Prewarming and refresh are real concerns; MCP startup is slow.
- **clew** — four transports (stdio / SSE / HTTP / DirectConnect).
- **vibe** — connectors and MCP are separate extension mechanisms with separate typed
  resource views.
- **MiMo** — MCP subcalls inside `exec` still call `ctx.ask()` individually, i.e. code mode
  does not batch away per-call approval.

## For cust

- Implement the `SKILL.md` standard; support loading from `~/.claude/skills` and
  `~/.codex/skills` from day one.
- Progressive disclosure, with an explicit `/skill:name` escape hatch.
- Once code mode exists, make skills importable modules — that is where the leverage is.
- Keep the extension surface a closed set and resist adding a ninth mechanism.
- If we do self-improvement: supplemental layer only, snapshots for rollback, base prompt
  immutable.
