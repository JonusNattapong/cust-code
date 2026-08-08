# cust-code

A Rust workspace implementing `cust`, a coding-agent CLI (provider-agnostic
chat/tool loop, sandboxed shell execution, and a terminal UI). Crates live
under `crates/`: `cust-code` (binary/CLI), `cust-core` (turn loop, slash
commands, compaction), `cust-provider` (LLM client), `cust-tools`/`cust-tools-api`
(tool registry), `cust-exec` (sandboxed command execution), `cust-session`
(saved-session persistence), `cust-tui` (terminal UI, mid-migration from
ratatui to an in-house `ink` renderer — see `crates/cust-tui/src/ink/mod.rs`).

Build/test: `cargo build --workspace`, `cargo test -p <crate> --lib`.

## Multiple concurrent sessions on cust-tui

More than one Claude Code session is often working in this repo at the same
time, and `crates/cust-tui` in particular is mid-migration (ratatui → `ink`,
tracked as phase 37e) — files like `banner.rs`, `statusline.rs`, `ui.rs`,
and files under `ink/components/` change hands frequently and are sometimes
mid-edit with transient compile errors that are not yours to fix.

When this happens:

- **Before editing a shared `cust-tui` file**, run `git status` and check
  whether it's already modified. A pre-existing diff there is very likely
  someone else's in-progress work, not leftover garbage.
- **Transient compile errors in files you didn't touch** (missing types,
  wrong function signatures, `MASCOT`/`visible_width`-style "not found in
  scope" errors) are usually another session mid-edit, not a bug to chase.
  Don't "fix" them speculatively — re-check the build after a beat; they
  often resolve themselves once that session finishes its edit.
- **Scope your own changes tightly.** Touch only the files your task
  actually requires. Don't refactor or "clean up" adjacent files someone
  else appears to be mid-edit on.
- **Before committing, `git status` again** and stage only the files your
  own change produced. Do not `git add -A` in this repo — it's how another
  session's unfinished work gets committed out from under them.
- **If the whole crate won't build** because of someone else's in-flight
  edit, verify your own change is correct in isolation (targeted
  `cargo check`/`cargo test -p cust-tui --lib <your::module>::` still
  surfaces real errors in your code) and say so plainly rather than
  papering over it or guessing at their intent.

## Sources you're free to draw on

`cust` is a from-scratch Rust rewrite, but it's explicitly informed by prior
art — don't reinvent something the survey already settled. When designing or
implementing a feature:

- **Read `D:\Projects\Github\clew-code\.reference\`** (sibling repo) — source
  snapshots of eight competitor coding agents (`codex`, `grok-build`,
  `kimi-cli`, `mistral-vibe`, `hermes-agent`, `MiMo-Code`, `openclaw`,
  `prime-agent`) plus `clew-code` itself, our own prior TS/Bun agent. This is
  the same material `DESIGN-NOTES.md` and `.knowledge/*.md` were written
  from — check those first, they're the distilled version; go to
  `.reference/` when you need the actual source, not just the summary.
- **Web search/fetch is fine** for anything current-events-shaped: library
  docs, API changes, protocol specs (e.g. ACP) — the reference snapshots are
  a point-in-time survey, not living docs.
- **Skills**: you can use an existing Claude Code skill, or write a new one,
  to help build or maintain `cust` itself. `~/.claude/skills/` and
  `clew-code/.claude/skills/` (e.g. `skill-creator`, `mcp-builder`) are
  fair game to borrow structure from. This is a tool for *doing the work*,
  separate from `.knowledge/07-skills-and-extensions.md`'s notes on `cust`
  someday implementing the `SKILL.md` standard for its own users.

## Keep this file current

Update this file whenever you land a change that shifts something it
describes — a new crate, a completed migration (e.g. once 37e finishes,
replace the "mid-migration" note above with the new steady state), a new
top-level system worth a newcomer knowing about. Small wording fixes don't
need their own pass; do it as part of the commit that made the fact stale,
not as a separate cleanup task later.
