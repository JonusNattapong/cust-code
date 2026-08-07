# Daemon, workers, and session storage

prime-agent's `daemon.md` is the most rigorous process-architecture document in the survey.
mistral-vibe's ADR 0006 is the best statement of what a session *file* owes you. Together
they cover the ground.

## Process topology (prime-agent)

```
clients (TUI · print · JSON · RPC)
        │  public local protocol (JSONL)
   supervisor  ──► catalog subprocess (saved-session scans)
        │
        ├─► resident worker: root A + scheduler + kernels + RLM descendants
        ├─► resident worker: root B + ...
        └─► client-owned worker: hidden root + ...
```

Ownership is strict, and the negative statement is the useful half:

- **Supervisor** owns public sockets, attachments, routing, agent-message delivery, worker
  health, command journals, coordinated updates. It **does not** execute providers, tools,
  compaction, bash, kernels, schedules, or transcript scans.
- **Worker** owns one root runtime, its session, scheduler, kernels, and every descendant
  below that root. New/switch/fork/import replace the root runtime *inside* the worker while
  preserving the public active-session ID.
- **Catalog subprocess** owns saved-session scans, so a scan failure fails one request
  instead of interrupting live workers.

Resident vs client-owned workers is a nice distinction: interactive sessions get resident
workers that survive client exit; print/JSON/RPC/`--no-session` get the same runtime with a
client-owned lifecycle — normal completion removes the worker, unexpected client loss starts
a bounded cleanup grace period, and reconnect with the same stable client identity cancels
cleanup.

Supervisor loss is handled without a single point of failure: workers monitor the public
socket, and if it disappears one worker acquires an **atomic launch lease** and starts a
replacement, which then adopts the live workers. Worker crash affects one root tree only;
recovery retries at 250 ms, 1 s, 5 s, then marks the root failed.

## Leases

Every persisted session is protected by a process-safe lease **keyed by canonical JSONL
path**.

- A worker acquires the target lease before opening a session.
- Runtime replacement acquires the new lease *before* releasing the old one.
- Concurrent opens return `session_already_active` **with the owning active-session ID** —
  an actionable error, not a generic failure.
- Concurrent creates for the same path converge on one worker launch.

This is how you stop a daemon worker and a one-shot CLI invocation from writing the same
transcript. Cheap to implement, impossible to retrofit calmly.

## Idempotency and crash recovery

The part most projects get wrong:

- Mutating commands are keyed by `clientId + commandId` and recorded in an append-only
  journal **before dispatch**.
- Repeating a completed command returns the stored result.
- A received command with no durable result is reported as **uncertain and is not replayed**.
- Reconnect retains the same command ID; clients acknowledge completed mutations so journal
  entries can be compacted.
- After a worker crash, recovery reaps the old process group and tracked detached bash
  trees, **appends a visible recovery marker to the transcript**, restores the root under the
  same active-session ID, and does not replay uncertain side effects.

"Uncertain, not replayed" is the correct default. Replaying a command that may already have
run is how an agent deletes something twice.

## Event cursors, replay, snapshots

- Every sequenced event belongs to a worker **generation**. Clients keep
  `{generation, sequence}` and present it on attach; the server reports whether the interval
  is complete, partial, or unavailable.
- A generation change invalidates comparison with the old sequence.
- **Missing replay is not fatal** — the attach snapshot is the durable recovery baseline.
  The client applies the snapshot, ignores duplicate or retired-generation events, and
  reports a resynchronized session.
- Large snapshots are encoded in the worker and streamed as opaque chunks (512 KiB target)
  through a bounded supervisor cache; file-backed above 4 MiB. **The supervisor never
  constructs a history-sized object.**

## Private transport and backpressure

Supervisor↔worker frames are binary: 4-byte JSON header length, 4-byte payload length, a
small JSON routing header, opaque payload bytes. Workers serialize a public event **once**;
the supervisor reads only the routing header and forwards the same buffer to eligible
clients. Assistant streaming uses compact start/delta/end privately so the growing message
is not re-transferred every delta.

Backpressure is **attachment-local**: a blocked client stops receiving incremental events,
others continue, the supervisor keeps no unbounded per-client queue, and after drain the
attachment catches up from its cursor or takes a fresh snapshot.

Private connections authenticate with per-worker tokens and are **fenced to the current
supervisor generation**, so an obsolete replacement supervisor cannot keep commanding an
adopted worker.

## Scheduling

- One scheduler per worker, jobs persisted per session in
  `session-artifacts/<id>/scheduled-jobs.json`. **No global cron file.**
- Due ticks are **claimed and advanced before prompt delivery**, so a crash does not replay
  an uncertain prompt.
- A still-active claim **coalesces** later missed ticks instead of building an unbounded
  backlog.
- Worker recovery marks uncertain claims interrupted, keeps the advanced schedule, and
  resumes future ticks only.

clew has the same shape in `services/autonomous/` (queue, leases max 3, cron, dead-letter,
daemon) and fires cron only while the REPL is idle.

## Session storage (mistral-vibe ADR 0006)

The decisions worth copying verbatim:

- **Append-friendly for ordinary message writes, atomic for metadata, tolerant of old
  transcript shapes through migrations.**
- **Private session storage and the public projection are different contracts.** Only the
  server reads or writes session files; clients get a lossy `PublicSessionState`, page
  history through opaque cursors, and use stable IDs. **"Public events are not a persistence
  format."**
- Rewind has two *explicit* modes:
  - **forked** — preserves the source session, attaches a new session from the selected prefix;
  - **in-place** — keeps the session identity, persists the truncated prefix, and the
    discarded suffix is **gone**.
  "Never infer in-place rewind from a missing option." Callers that do not expose a choice
  must preserve the source session.
- Every rewind result is an **authoritative state replacement** — clients replace their
  projection from the response rather than editing visible history locally.
- Be honest about recovery limits: a reconnect to the same live harness recovers its
  snapshot and open callbacks; a **new process does not restore an in-flight turn** from
  JSONL. "Do not present live reconnect behavior as crash recovery."

prime-agent's on-disk layout, for reference:

```
~/.prime/agent/
  sessions/<root-session-id>.jsonl
  session-artifacts/<root-session-id>/
    kernel-state.dill · kernel-state.json · scheduled-jobs.json
    harness/harness_state.json
    sub-xxxxxxxx/<child-session-id>.jsonl
```

Artifact files are created only when their feature is used; non-persistent sessions go to
OS temp and gain no revivable artifacts.

## The honesty clause

Every project says a version of this, and so should we:

> Workers and kernels are separate processes for lifecycle and failure containment, **not**
> security sandboxes. They normally run with the same operating-system permissions as the
> client.

Process isolation is not a trust boundary. Saying so plainly is better than letting users
infer safety that is not there.
