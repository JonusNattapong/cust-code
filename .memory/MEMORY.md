# Memory

Durable facts about this project that an agent should recall at the start of a session.
Short and true. If it is derivable from the code or git history, it does not belong here.

Index:

- [DECISIONS.md](DECISIONS.md) — decisions made, with their reasons and their reversibility
- [PREFERENCES.md](PREFERENCES.md) — how the owner wants this project worked on

---

## Identity

- **Repo:** `D:\Projects\Github\cust-code`, branch `main`, no remote yet.
- **Crate** `cust-code`, **binary** `cust`. The two are deliberately different — the crate
  name only avoids a crates.io collision with the CUDA `cust` crate.
- Started 2026-08-07 by jonusnattapong, who also maintains `clew-code`.
- Previous names, both rejected as hard to remember: `hawser`, then `tug`.

## Where things live

| Path | Holds |
|---|---|
| `.knowledge/` | The study of nine other agents, done before any code was written |
| `.learning/` | Lessons and errors from building *this* project |
| `.memory/` | This — durable facts, decisions, preferences |
| `.doplan/` | The working task list for the current phase |
| `PLAN.md` | Phase order and the per-phase smoke test |
| `DESIGN-NOTES.md` | Condensed form of `.knowledge/` |

Source material for `.knowledge/` is `D:\Projects\Github\clew-code\.reference\` — eight
agent repos, untracked, not part of this repo.

## Open questions that block work

1. **Is `cust` a rewrite of clew, a companion, or unrelated?** Unanswered. It decides whether
   session formats need to interoperate, and whether clew features should be ported or
   redesigned.
2. Code mode only, or a small direct tool set alongside it? (codex and MiMo ship both.)
3. Can subagents be spawned from inside code mode? Decides whether a typed host-request
   bridge is needed at Phase 4 rather than Phase 9.

## Current state

Phase 0 complete: cargo scaffold, `cust help` / `cust version`, 4 integration tests running
the real binary, full gate green. No model call exists yet.
