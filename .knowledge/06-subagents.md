# Subagents, messaging, goals, autonomy

## The two delegation philosophies

**grok / vibe / clew — subagents are a tool.** `spawn_subagent` starts a child session with
its own context window and a toolset from its agent type; it reports a summary back when it
finishes.

**prime-agent — subagents are a function call inside the interpreter.**

```python
handle = await rlm("Review the authentication flow", name="auth-reviewer")
```

The trade is explicit. Delegation-as-code composes (you can loop, filter, fan out in a single
turn), but it needs a typed host-request bridge, a depth limit, and a durable child registry.
MiMo went the other way and deliberately excluded control-flow tools from `exec`.

## prime-agent's model in detail

The design decision that makes it work:

> The call returns immediately after task admission with a child handle; **it never waits for
> or returns the child's answer.**

`RLMSpawnHandle` = `{rlm_child_id, name, session_dir, model}` — admission only. Results arrive
solely through explicit `agent_message` replies or files. So the parent spawns three children
and **ends its turn** rather than blocking:

```python
api  = await rlm("Review the public API", name="api-reviewer")
test = await rlm("Review the test coverage", name="test-reviewer")
```

Child execution sequence:

1. check `RLM_DEPTH < RLM_MAX_DEPTH` (default max depth 1 — root may spawn, children may not);
2. resolve requested model or inherit parent's;
3. create `sub-xxxxxxxx/` under the parent artifact directory;
4. admit into the parent registry, return the handle;
5. *then*, detached, create the child SessionManager/Agent/AgentSession;
6. reuse provider hooks, resource loader, model registry, tools, transport, retry, thinking config;
7. run, retain the session, update lifecycle independently;
8. attribute child usage to the parent assistant turn and persist it.

**Fail loud on model selection:** `model` must be an exact `provider/model` from
`rlm.find_models()`; unknown options fail rather than being ignored; if the model is
unavailable or fails auth preflight, **spawn fails instead of silently falling back**.

**Registry survives everything.** `rlm.list_subagents()` returns child IDs, active-session
IDs, session IDs, names, directories, status — and survives kernel restart, compaction, and
parent restore. Deleting writes a durable tombstone and removes the child from messaging and
observation, but does not erase transcripts on disk. Registry scope follows the parent
transcript; an unrelated new parent does not inherit children.

**Usage accounting.** The parent transcript persists a `child_usage_attributed` entry with
the target message ID, the child usage, and the resulting aggregate; on reload the aggregate
is reapplied. Context-tree reporting subtracts attributed child usage when showing a node's
own usage. Net effect:

> Child work increases billable session totals but **does not inflate the parent model's
> context-window measurement.**

That distinction — cost aggregates up, context does not — is easy to get wrong and produces
either bogus bills or premature compaction.

## Agent-to-agent messaging

prime-agent routes direct messages between active sessions and retained subagents, with
**delivery modes**:

| Mode | Behavior |
|---|---|
| `auto` | steer a busy target; deliver immediately to an idle one |
| `steer` | intentionally inject into active work |
| `follow_up` | wait until the target's current work finishes |

Receipts are `delivered` (reached an idle target's context) or `queued`. Broadcast is limited
to the family roster. The **daemon derives sender identity** — a sender cannot claim to be
someone else — and enforces message-size, rate, and pending-queue limits.

`receiver_role` is `parent` / `child` / `sibling` plus a name, which is a nicer addressing
scheme than raw session IDs.

## grok: agents vs personas, and uniform task handles

Two axes instead of one:

| | Agent | Persona |
|---|---|---|
| Configures | whole session: model, tools, prompt mode, system prompt | behavioral overlay on a subagent's prompt |
| Scope | primary session or subagent | subagents only |
| Defined in | `.grok/agents/*.md` | `config.toml` `[subagents.personas]` or `.grok/personas/*.toml` |
| Controls | model, tool availability, prompt body, skills | tone, output format, task focus, I/O contracts |

> An agent defines the session itself. A persona shapes how a subagent behaves within a
> session.

And **one uniform handle for background shell jobs and subagents**:

- `run_terminal_command(background: true)` → `task_id`
- `get_command_or_subagent_output(task_id, timeout_ms?)`
- `wait_commands_or_subagents(task_ids, mode=wait_any|wait_all, timeout_ms)` — max 20
- `kill_command_or_subagent(task_id)` — SIGTERM then SIGKILL for shells; Cancel then Shutdown
  for subagents

Treating "a running thing" as one concept rather than two is a small design choice that
removes a whole category of duplicated API.

## Goals vs autonomous mode — different things

prime-agent separates them cleanly:

- A **goal** is a durable objective the harness re-presents across turns until complete,
  paused, budget-limited, errored, or cleared. It records token usage, elapsed time,
  continuation count, and an optional token budget. **Only `goal.complete()` marks success.**
  Creating one is an explicit user or host action, "not something the agent should infer from
  every task."
- **Autonomous mode** is a bounded *policy*: keep injecting continuations until quality gates
  pass or a continuation / turn / token / wall-clock limit is hit. A failed gate returns
  bounded output to the agent for another attempt, and **a gate is not re-run when the
  workspace has not changed**.

```bash
prime-agent --autonomous --autonomous-gate "npm run check" --autonomous-max-turns 20 "..."
```

The README's caveat is the honest bit:

> A passed gate checks only what that gate verifies; **reaching a limit does not imply task
> success.**

Three scheduling surfaces, deliberately distinct:

| Surface | Owner | Purpose |
|---|---|---|
| `/heartbeat` | user | one visible recurring instruction for this session |
| `rlm_heartbeat` | agent | several programmatic recurring instructions, internal |
| `schedule` | user or automation | general one-time or cron prompts targeted at an agent |

The agent's heartbeats cannot replace or clear the user's. Small boundary, prevents a whole
class of confusing behavior.

## clew's equivalents

Five execution layers by intent: Agent (main) / Subagent (Explore) / Teammate (swarm) /
LAN Peer (`/peer`) / Background daemon. Plus `ScheduleFollowup` — the agent parks unfinished
work with a summary, remaining steps, and `delayMinutes`, re-enqueuing to itself as a
one-shot cron. Cron fires only while the REPL is idle.

## For cust

- Subagents outside code-mode first (MiMo's line). Revisit once the host-request bridge
  exists.
- Admission handle, not a blocking call.
- Cost aggregates to the parent; context does not.
- Uniform task handle for shell jobs and subagents.
- Goal and autonomous mode are separate features with separate state.
