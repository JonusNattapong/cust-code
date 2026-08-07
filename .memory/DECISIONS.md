# Decisions

What was decided, why, and how expensive it would be to reverse. Newest first. When a
decision is reversed, keep the entry and add the reversal — the reasoning is the value.

---

## D-007 · Reuse clew's credentials, read-only

**Decision.** Read API keys from `~/.clew/.credentials.json`, `~/.clew/provider.json`, and
the project `.env`. Never write to any of clew's files. On a missing key, fail with a message
naming the file we looked in rather than falling back silently.

**Why.** The owner already has working credentials there, and importing another agent's auth
is an established move — MiMo-Code offers one-step import from Claude Code. Read-only means
we can never corrupt a working clew setup.

**Reversibility.** Easy. It is one loader behind the config layer.

## D-006 · Name is `cust-code` / binary `cust`

**Decision.** After `hawser` and `tug`, settle on `cust-code` with binary `cust`.

**Why.** Owner's choice, matching the `clew-code` pattern. `cust` alone collides with a real
crates.io crate (CUDA Driver API bindings, 455k downloads); the `-code` suffix avoids it
while the binary stays short. Noted at the time: readers may parse "cust" as an abbreviation
of "customer"; owner accepted this.

**Reversibility.** Easy now (no remote, no users), expensive after publishing.

## D-005 · QuickJS via `rquickjs` for code mode

**Decision.** Embed QuickJS. Revisit `deno_core` (V8) only if QuickJS proves limiting.

**Why.** codex uses V8 and MiMo uses QuickJS for the same job, so both are proven. QuickJS is
small, embeddable, and adds no V8 build to CI. Python was rejected: prime-agent needs `uv`, a
bootstrapped venv, `ipykernel`, and a ZeroMQ Jupyter transport — too many moving parts for a
single static binary.

**Reversibility.** Moderate. Contained in `cust-codemode` if the host-tool bridge is kept
engine-agnostic. Design the bridge that way deliberately.

## D-004 · Adopt ACP; do not invent an editor protocol

**Decision.** Speak the Agent Client Protocol for editor integration.

**Why.** Five of nine surveyed projects implement it (kimi, vibe, hermes, grok, openclaw) and
the other two large ones have equivalents. An agent that does not speak ACP cannot be used
from Zed or JetBrains.

**Reversibility.** Easy to add, hard to remove once editors depend on it.

## D-003 · Permission lives in the tool contract

**Decision.** The `Tool` trait carries a `permission()` method; permission is not a check the
caller remembers to run. Shell permission analysis recurses into nested command constructs
from day one.

**Why.** mistral-vibe ADR 0004. A permission check that matches the top-level command string
is defeated by `sh -c`, backticks, `xargs`, or a pipeline. Retrofitting this means auditing
every callsite.

**Reversibility.** Very hard. This is why it is decided before Phase 2.

## D-002 · Twelve crates, drawn on the seams both Rust agents found

**Decision.** `cust-code`, `-core`, `-config`, `-config-types`, `-provider`, `-tools-api`,
`-tools`, `-exec`, `-codemode`, `-session`, `-proto`, `-tui`.

**Why.** codex (~120 crates) and hermes (one flat 200-module package) bracket the failure
modes. The specific splits — config vs config-types, tools-api vs tools — are ones codex and
grok arrived at independently, and both are expensive to retrofit.

**Reversibility.** Splitting later is expensive; merging later is cheap. So start split.

## D-001 · Rust, edition 2024

**Decision.** Write it in Rust. (First scaffold was TypeScript/Bun; changed on the owner's
instruction before any real code existed.)

**Why.** Owner's call. It also removes clew's `.js` shadow-file class of bug entirely —
stale compiled files shadowing source at runtime, which clew polices with a pre-commit hook.

**Reversibility.** None, practically.
