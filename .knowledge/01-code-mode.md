# Code mode — the convergence

The strongest finding in the survey. Four projects, no shared code, same conclusion:

> Stop giving the model N tool schemas. Give it a sandboxed interpreter where the tools are
> functions.

| Project | Engine | Interface | State between calls |
|---|---|---|---|
| codex `code-mode` / `exec` | **V8 isolate**, fresh per call | `await tools.exec_command(...)`, `ALL_TOOLS` | `store(k,v)` / `load(k)` |
| MiMo-Code `exec` | **QuickJS** | `tools.<name>()` | none |
| prime-agent `ipython` | **IPython kernel**, persistent | any Python; `rlm()`, skills as imports | full namespace, survives compaction |
| hermes | Python + RPC | scripts call tools over RPC | — |

## Why

codex's own tool description states the motive plainly: *"Run JavaScript code to
orchestrate/compose tool calls."* A five-step pipeline that costs five model round-trips as
individual tool calls costs **one** as a script. MiMo's doc puts the division of labour as:

> The model decides what to do, `exec` determines how to compose the operations, and the
> host decides whether they are allowed and how their side effects are produced.

## The three things that make it safe

Not visible from the outside, and each appears in more than one project.

### 1. The interpreter has no capabilities — the host does

- **codex**: "Runs raw JavaScript — no Node, no file system, no network access, no console."
- **MiMo**: QuickJS guest with no Node, `process`, `fetch`, timers, or module loading.

Every real side effect goes back **out** to a host tool that runs the normal permission
path. MiMo names the two boundaries explicitly:

1. `evalScript()` isolates guest code with QuickJS;
2. actual side effects are performed by host tools and pass through permissions,
   external-directory checks, memory guards, and each tool's own validation.

And it is honest about what is *not* covered: "QuickJS isolates only the `exec` code.
`bash` remains a real shell, not a container sandbox."

**The sandbox isolates code, not effects.** If you forget this you will build a sandbox that
guards the wrong thing.

### 2. Late-bound tool registry

MiMo's `tool-script-ref.ts` hands `exec` the **same `Tool.Def` instances** the outer layer
received, after model/agent filtering. Consequences:

- `read`, `write`, `edit` hidden from the outer layer do not reappear inside `exec`;
- built-in subcalls execute the original `Tool.Def.execute()` with the original context;
- MCP subcalls still call `ctx.ask()` individually;
- `exec_command` is only an alias for `bash`, same permissions, same execution path.

Without late binding, code-mode is a permission bypass wearing a sandbox costume. This is
the single most important implementation detail on the page.

### 3. Control-flow tools are excluded

MiMo keeps `task`, `actor`, `question`, `skill`, `workflow`, `cron`, and `session` out of
`exec` — they change conversation or scheduling state and "should not be hidden inside a
single script call."

prime-agent deliberately goes the **other way**: `rlm()` spawns subagents from inside
Python. It pays for that with a whole typed host-request bridge (`rlm.host_request(...)`
over a Jupyter comm), a depth limit, and a parent-scoped child registry. Both choices are
defensible; the cost of the permissive one is a day-one bridge.

## The yield protocol

A long script cannot block a turn. codex's design:

- `exec` returns `Script running with cell ID ...` when it does not finish in time;
- `wait(cell_id, yield_time_ms, max_tokens, terminate)` pulls new output since the last
  yield, or the final result; it may yield again with the same `cell_id`;
- `yield_control()` flushes accumulated output to the model while the script keeps running;
- a first-line pragma sets per-call budgets: `// @exec: {"yield_time_ms": 10000, "max_output_tokens": 1000}`;
- when the script finishes, the isolate dies and **unawaited promises are silently discarded**.

## Resource limits (MiMo's numbers, a good starting point)

| Resource | Default / max |
|---|---|
| Nested tool calls | 50 default, 500 max |
| Concurrent calls | 8 |
| Active computation | 60s default, 600s max |
| Wall clock | 30 min |
| Guest memory | 64 MiB |
| Code / return / logs | 128 KiB / 256 KiB / 64 KiB |
| Single file via `files.*` | 10 MiB |

MiMo also constrains the guest's own file helpers: `files.readText` reads UTF-8 only, within
the worktree or OS temp; `files.writeText` writes **only** to OS temp. Project changes must
go through permission-controlled host tools.

## Other guest globals codex exposes

`exit()`, `text(v)`, `image(...)`, `audio(...)`, `generatedImage(...)`, `notify(v)`
(injects an extra tool output immediately), `setTimeout`/`clearTimeout` (pending timeouts do
not keep `exec` alive), and `ALL_TOOLS` metadata so the model can discover deferred tools by
filtering rather than having every schema in context.

That last point matters: **code mode doubles as a tool-discovery mechanism.** Instead of
N schemas in the system prompt, you ship a searchable catalogue.

## Both, not either

codex and MiMo both keep a **small direct tool set alongside `exec`**. MiMo's GPT profile is
exactly four tools — `bash`, `apply_patch`, `view_image`, `exec` — and hides `read`, `write`,
`edit`, `multiedit`, `grep`, `glob`, `notebook_edit` because `exec` subsumes them. So the
real design question is not "schemas or code mode" but "which few tools stay direct."

MiMo notes its own unfinished edge here: prompt routing (`gpt.txt`/`codex.txt`/`beast.txt`)
and tool-profile selection are two separate sets of string rules keyed off the model ID,
"not yet unified into a model-capability negotiation layer." A known-good thing to design
properly the first time.

## What this means for a Rust agent

codex and MiMo both prove the shape: **Rust/TS host + embedded JS guest**. For `cust`:

- `rquickjs` (QuickJS bindings) to start — small, embeddable, no V8 build in CI.
- `deno_core` (V8) if QuickJS proves limiting.
- **Not** Python: prime-agent needs `uv`, a bootstrapped venv, `ipykernel`, and a ZeroMQ
  Jupyter transport with HMAC-signed multipart framing to make it work. That is a lot of
  moving parts to ship inside a single static binary.

## prime-agent's Python variant, for completeness

Worth understanding even though we are not copying it, because the persistence properties
are genuinely better:

- Python state survives across tool calls **and across compaction** — variables, imports,
  functions, parsed results, task handles.
- `%%bash` cells are temporary subshells; Python state and `%cd` persist in the kernel.
- Kernel namespace can be snapshotted (`kernel-state.dill`) into the session artifact
  directory for revival.
- Host requests reply on the Jupyter **control** channel, not shell — replying on shell
  would deadlock, because the active `execute_request` cannot finish until the response
  arrives and the kernel will not process the shell response until the request finishes.
  That deadlock is a real trap for anyone building a comparable bridge.
- `KernelManager.execute()` is serialized: one kernel, one namespace, no two cells at once.
  Child agents still run concurrently because each delegation gets its own comm and runtime.
