# Module and crate boundaries

The survey spans both failure modes, which makes the middle easy to locate.

## Two failure modes

**Too many boundaries — codex, ~120 crates.** `agent-graph-store`, `agent-identity`,
`analytics`, `ansi-escape`, `apply-patch`, `app-server`, `app-server-client`,
`app-server-daemon`, `app-server-protocol`, `app-server-protocol-noop-macros`,
`app-server-test-client`, `app-server-transport`, … through to `v8-poc` and
`windows-sandbox-rs`. Seven crates for the app-server alone. This is a Bazel-scale answer to
a Bazel-scale problem: hundreds of contributors, remote build execution, strict caching. The
crate count is a *build system* decision as much as an architecture one.

**Too few — hermes, one flat `agent/` package of ~200 modules.** `conversation_loop.py` next
to `tts_registry.py` next to `billing_links.py` next to `ssl_guard.py`. It clearly works and
ships fast, but nothing prevents any module from importing any other, so the boundaries exist
only in people's heads.

**The middle:** grok's ~60 crates and prime-agent's 4 packages both read as deliberate.
mistral-vibe's `vibe/core` + surfaces is the clearest small structure.

## The seams both Rust projects independently drew

1. **Provider/model is its own crate.** codex: `model-provider`, `model-provider-info`,
   `models-manager`. grok: `xai-grok-models`, `xai-grok-sampler`, `xai-grok-sampling-types`.
   HTTP and auth details must not leak into the agent loop.
2. **config split from config-types.** grok has both `xai-grok-config` and
   `xai-grok-config-types`, so leaf crates depend on the types without pulling loading logic.
   Cheap; prevents a dependency knot that a single `config` crate always creates.
3. **tools-api split from tools.** grok: `xai-grok-tools-api` (traits, schemas) vs
   `xai-grok-tools` (implementations, `registry/`, `implementations/`, `reminders/`).
   codex: `tools` + `core-plugins`. Retrofitting this is expensive.
4. **Shell/exec is several crates.** codex: `shell-command`, `exec`, `exec-server`,
   `exec-server-protocol`, `shell-escalation`. grok: `xai-grok-shell`, `-shell-base`,
   `-shell-session-support`, plus `ptyctl` / `ptyctl-cli` for PTY control.
5. **The daemon/editor protocol is separate from the CLI.** codex `app-server*`,
   grok `xai-acp-lib`, openclaw `acp-core` + `gateway-protocol`.
6. **Git is a subsystem, not shell-outs.** grok: `xai-gix-status` (in-process `gix`),
   `xai-hunk-tracker`, `xai-fast-worktree`, `xai-fsnotify`. Spawning `git status` every turn
   is a measurable cost in an interactive loop.
7. **Codebase graph as a crate.** grok `xai-codebase-graph`; clew has CodeGraph. Structural
   code intelligence is its own thing, not a search helper.

## codex's Rust style rules (`AGENTS.md`)

The best set in the survey; adopt largely verbatim.

- **Modules under 500 LoC excluding tests.** Past ~800, add a new module rather than
  extending. Named high-touch offenders — `tui/src/app.rs`, `chatwidget.rs`,
  `bottom_pane/mod.rs` — because central orchestration files attract unrelated changes.
- When extracting from a large module, **move the related tests and docs with it** so
  invariants stay near the code that owns them.
- **Exhaustive `match`; avoid wildcard arms.**
- **No `#[async_trait]`, no `#[allow(async_fn_in_trait)]`.** Prefer RPITIT with an explicit
  bound: `fn foo(&self, …) -> impl Future<Output = T> + Send;` Implementations may still be
  `async fn` when they satisfy the contract.
- **Avoid bool or ambiguous `Option` parameters** that force callers to write `foo(false)` /
  `bar(None)`. Prefer enums, named methods, or newtypes so the callsite self-documents. Where
  unavoidable, an exact `/*param_name*/` comment before the literal — enforced by a lint.
- Inline `format!` args; method references over closures; collapse `if`s.
- **Prefer private modules with an explicitly exported public crate API.**
- **Do not create small helper methods referenced only once.**
- New traits get doc comments explaining their role and how implementations should use them.
- Instrument with `#[tracing::instrument]` on the definition, not `.instrument()` at call
  sites — and check whether the callee is already instrumented first.
- Tests: compare whole objects, not field by field. No tests for statically defined values.
  No negative tests for removed logic.

## Composition roots

grok's binary crate `xai-grok-pager-bin` is explicitly a "composition-root package" and the
workspace root `Cargo.toml` is **generated** — "treat it as read-only. Prefer editing
per-crate `Cargo.toml` files."

Development guidance that follows from a large workspace: `cargo check -p <crate>` — "always
target specific crates; full-workspace builds are slow."

## Names and namespacing

- codex: every crate prefixed `codex-`; the `core` folder's crate is `codex-core`.
- grok: `xai-` prefix, binary artifact `xai-grok-pager`, **shipped as `grok`**.

Confirming that the crate name and the command users type are independent — which is why a
taken crates.io name is a naming inconvenience, not a blocker.

## clew's cautionary tale

One flat `src/` with 78 tools and 114 commands, in TypeScript. Two documented consequences
worth carrying:

1. **A ~1866-error typecheck baseline that has been red "for a while."** Once the gate is
   red, it stops being a gate; AGENT.md has to teach contributors how to tell their errors
   from the ambient ones.
2. **`.js` shadow files** — stale compiled files shadowing `.ts` at runtime, requiring a
   pre-commit hook (`check-shadow-pairs.sh`) to police. Rust's module system makes this class
   of bug impossible, which is a real argument for the language choice.

The lesson is not "TypeScript bad" — it is that a build gate you cannot keep green stops
protecting anything, so keep the project small enough that the gate stays green.

## Target for cust

Twelve crates (see [../PLAN.md](../PLAN.md)), each corresponding to a seam above:

```
cust-code  cust-core  cust-config  cust-config-types  cust-provider
cust-tools-api  cust-tools  cust-exec  cust-codemode  cust-session
cust-proto  cust-tui
```

Adopt codex's style rules in `AGENTS.md` before the first real feature lands — they are much
cheaper to follow from the start than to apply retroactively.
