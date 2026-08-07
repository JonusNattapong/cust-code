# Do-plan

The working task list. `PLAN.md` says *what phases exist and why*; this says *what is being
done right now and what is left in the current phase*.

One file per phase. Check items off as they land, and move the file to `done/` when the
phase's smoke test passes for real.

| File | Phase | State |
|---|---|---|
| [phase-0-scaffold.md](phase-0-scaffold.md) | 0 — Scaffold | ✅ done |
| [phase-1-talk-to-a-model.md](phase-1-talk-to-a-model.md) | 1 — Talk to a model | 🔜 next |

## Rules

- A task is done when the behavior works when run, not when the code compiles.
- If a task turns out to be blocked, write why in the file rather than deleting it.
- Anything discovered along the way that is a *lesson* goes to `.learning/`, not here.
- Anything decided along the way goes to `.memory/DECISIONS.md`.
