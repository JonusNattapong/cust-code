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
| `ARCHITECTURE.md` | The synthesized design — what we are building, and what we took from whom |
| `PLAN.md` | Phase order and the per-phase smoke test |
| `DESIGN-NOTES.md` | Condensed form of `.knowledge/` |

Source material for `.knowledge/` is `D:\Projects\Github\clew-code\.reference\` — eight
agent repos, untracked, not part of this repo.

## The framing decision

**`cust` is a new design, not a clew rewrite** (owner, 2026-08-08). Take the best answer
from each surveyed project; inherit no codebase. clew's *lessons* carry over, clew's
*structure* does not. The synthesis — including which subsystem comes from which project —
is `ARCHITECTURE.md`.

Nothing is blocking. Two things to settle when we reach them: `~/.cust/` vs
`~/.config/cust/` (Phase 1), and whether to ship a clew session importer at all (after
Phase 5).

## Current state

Phase 0 complete: cargo scaffold, `cust help` / `cust version`, 4 integration tests running
the real binary, full gate green. No model call exists yet.
