# Sandboxing and process hardening

grok-build has the cleanest user-facing model; codex has the widest platform coverage.

## grok-build: named profiles, kernel-enforced

Landlock on Linux, Seatbelt on macOS. **Off by default.**

| Profile | FS read | FS write | Child network | Use |
|---|---|---|---|---|
| `off` | unrestricted | unrestricted | unrestricted | no sandbox |
| `workspace` | everywhere | CWD + `~/.grok/` + temps | allowed | normal development |
| `devbox` | everywhere | all top-level except `/data` | allowed | disposable dev VMs |
| `read-only` | everywhere | `~/.grok/` + temps | blocked¹ | exploration, review |
| `strict` | CWD + system paths | CWD + `~/.grok/` + temps | blocked¹ | untrusted code |

¹ **Linux only** (seccomp). On macOS it is a documented no-op.

That footnote is the thing to copy as much as the table. A limitation stated is worth more
than a limitation implied.

### Custom profiles

`~/.grok/sandbox.toml` (global) or `.grok/sandbox.toml` (per project):

```toml
[profiles.project]
extends = "workspace"
restrict_network = true
read_only  = ["/data"]
read_write = ["/tmp/scratch"]
deny       = ["/data/shared-secrets", "**/.env", "**/*.pem"]
```

`deny` is kernel-enforced for **read and write/rename**, and supports globs. This is the
right answer for "keep the agent out of my credentials" — a policy the kernel enforces, not
a string check in the tool layer.

### Self-protection — the subtle part

Under `workspace` / `read-only` / `strict`, the state directory stays writable for session
files, but the kernel **write-denies the paths used as user-global hook sources** (they stay
readable):

- `~/.grok/hooks/`
- `~/.grok/hooks-paths` (a registry file; only its absolute targets are loaded as hooks)
- absolute targets listed in `hooks-paths`

Plus:

- On first launch under these profiles, real empty `hooks/` and `hooks-paths` are created if
  missing — **never symlinks or wrong types**.
- A symlinked `$GROK_HOME`, or a `hooks-paths` entry with a symlink component, **refuses
  sandbox start** (prevents retargeting).
- Existing parent directories of protected paths are **pinned so they cannot be renamed out
  from under the deny**; siblings stay writable.
- On Linux, nested user namespaces are disabled inside bubblewrap so mount binds cannot be
  rearranged.
- Profiles that require this protection **refuse to start if the kernel policy cannot be
  applied**, including Linux without verified read-only mounts.
- `devbox` deliberately opts out (disposable VMs).

The threat model here is worth naming: without it, an agent that can write its own hook
directory can install persistence that survives the sandbox. Fail-closed on start is the
right call.

## codex: platform work split across crates

`linux-sandbox`, `windows-sandbox-rs`, `bwrap`, `execpolicy`, `sandboxing`,
`process-hardening`, plus `network-proxy`, `net-policy`-style approval in
`core/src/tools/network_approval.rs` and `network_policy_decision.rs`.

Sandboxing is never "a flag in the exec function". It is several crates and a policy
language (`execpolicy`).

codex's `AGENTS.md` also shows how the sandbox leaks into test design: env vars
`CODEX_SANDBOX_NETWORK_DISABLED=1` and `CODEX_SANDBOX=seatbelt` exist so tests can early-exit
when they cannot run under the sandbox they are testing. Worth planning for — integration
tests for a sandbox cannot themselves run inside it.

## MiMo: two boundaries, stated separately

1. QuickJS isolates guest code (no Node, `process`, `fetch`, timers, modules);
2. side effects go through host tools with permissions, external-directory checks, memory
   guards, and per-tool validation.

And the honest limit: "`bash` remains a real shell, not a container sandbox."

## mistral-vibe: analysis at the right level

ADR 0004 — shell permission analysis covers **semantic execution**, not top-level command
text:

- recursively inspects nested command constructs;
- normalizes shell-native paths before workspace-boundary checks, **including MSYS drive
  paths on Windows**;
- auto-allowlisted readers still require approval when their path arguments leave the
  workspace.

Guidance: "keep path-inspection coverage at least as broad as shell reader allowlists, and
test nested commands and platform-specific path forms."

A permission check that pattern-matches `rm -rf` on the outer command is defeated by
`sh -c`, backticks, `xargs`, or a pipeline. This is the correct paranoia level.

## Consensus honesty clause

Every project states some version of:

> This is a durable control environment, **not** a security sandbox. Review third-party
> skills and use an external sandbox for untrusted repositories and instructions.

Trusted-by-design components: installed packages, skills, extensions, MCP servers. The
sandbox protects against accidents and blast radius, not against a hostile skill you chose
to install.

## For cust

- Named profiles (`off` / `workspace` / `read-only` / `strict`) with `extends` and a glob
  `deny` list, in `sandbox.toml`.
- Landlock + Seatbelt; **document Windows honestly** rather than shipping a profile that
  silently does nothing.
- Write-deny our own hook/config-execution paths; refuse to start on symlinked home.
- Fail closed: if the kernel policy cannot be applied for a profile that requires it, do not
  start.
- Shell permission analysis recurses into nested constructs from day one — retrofitting this
  means auditing every callsite.
