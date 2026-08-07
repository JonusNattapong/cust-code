# Learnings

Things discovered **while building cust-code** that were not obvious beforehand. One entry
per lesson, newest first. Append when something surprises you; do not rewrite history.

Distinct from the sibling directories:

- `.knowledge/` — what we learned *from other projects*, before writing code.
- `.learning/` — what we learn *from our own code*, while writing it.
- `.memory/` — durable facts and decisions an agent should recall next session.
- `.doplan/` — the working task list for the current phase.

---

## 2026-08-08 — crates.io squatting is not a naming constraint

Every plausible short name is taken on crates.io (`tug`, `clew`, `rope`, `knot`, `bosun`,
`ropewalk` — all of them, most with no real project behind them). This nearly drove a bad
naming decision.

It does not matter. The crate name and the command users type are independent:

| Project | Crate | Binary |
|---|---|---|
| grok-build | `xai-grok-pager-bin` | `grok` |
| codex | `codex-*` (every crate prefixed) | `codex` |
| cust-code | `cust-code` | `cust` |

**Applied:** pick the name for memorability and check GitHub, not crates.io. Publish under a
suffixed crate name if we ever publish at all.

## 2026-08-08 — rust-analyzer holds a directory handle and blocks rename on Windows

Renaming the repo folder failed with `Permission denied` from `mv`, PowerShell
`Rename-Item`, and after deleting `target/`. Two `rust-analyzer.exe` processes were holding
handles.

**Applied:** on Windows, `Copy-Item -Recurse` to the new name, then remove the old directory
— the delete succeeds once nothing new is being opened under it. Do not kill the user's
editor processes to win a rename.

## 2026-08-07 — the survey changed the plan, so it was worth doing first

The initial design notes were written after skimming layouts and got the emphasis wrong —
they treated code mode as one idea among several. Reading the actual architecture docs showed
four independent teams had converged on it, which promoted it from "interesting" to the
central design decision, and moved it up the plan.

**Applied:** `PLAN.md` phases the interpreter at Phase 4, before sessions and TUI, because
the tool contract has to be shaped for it from the start. Retrofitting a late-bound registry
onto an existing tool layer is the expensive version.
