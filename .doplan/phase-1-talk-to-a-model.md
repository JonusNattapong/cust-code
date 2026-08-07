# Phase 1 — Talk to a model 🔜

Goal: the smallest thing that is actually an agent — one prompt in, one streamed answer out.
No tools, no session, no TUI.

## Decide first

- [ ] Which provider first? Pick whichever key `~/.clew/.credentials.json` already holds.
      Confirm the file's actual schema at implementation time — it was not readable during
      planning (blocked, correctly, by the permission classifier), so **do not assume a
      shape**. Ask the owner to confirm, or read it with their approval.
- [ ] `~/.cust/` or `~/.config/cust/` for our own config? (grok uses `~/.grok/`, prime uses
      `~/.prime/agent/` with an XDG fallback when the home path is not writable.)

## Tasks

### `cust-config-types` + `cust-config`

- [ ] `Config` type; no loading logic in the types crate
- [ ] Layered precedence: defaults → `~/.cust/config.toml` → `./.cust/config.toml` → env → flags
- [ ] Report a bad config file without crashing the whole app where it is safe to continue
      (vibe ADR 0007)

### Credentials

- [ ] Read-only loader for `~/.clew/.credentials.json`, `~/.clew/provider.json`, project `.env`
- [ ] Never open clew's files for writing — enforce it in the loader's type, not by convention
- [ ] Missing key → error naming the exact file we looked in. No silent fallback.
- [ ] Redact key material from every error, log, and debug output

### `cust-provider`

- [ ] One provider, streaming
- [ ] Model capabilities declared: `chat` / `vision` / `tool_calling` / `streaming` / `max_context`
- [ ] Prefer the live `/models` context window over the static table; static is the cold-cache
      fallback (clew's lesson — the compaction threshold depends on this number)
- [ ] Typed usage capture; if the provider only reports limits in response headers, capture
      opportunistically and never probe
- [ ] **No process-global session model or provider.** Publish an atomic snapshot; each run
      forks from it (openclaw's generations; clew's `setSessionProvider` warning)

### CLI

- [ ] `cust ask "<prompt>"` — non-interactive, streams to stdout
- [ ] Ctrl-C cancels cleanly mid-stream

### Housekeeping

- [ ] `.gitattributes` with `* text=auto eol=lf` to stop the CRLF warnings
- [ ] `AGENTS.md` with codex's Rust style rules (module size, exhaustive match, no
      `#[async_trait]`, no bool positional params) before the codebase grows

## Smoke test

```bash
cust ask "reply with the single word: ok"     # streams `ok`, exits 0
```

Then unset the key and confirm the error names the file it looked in. Then set an invalid
key and confirm the auth error is legible and does not leak the key.

## Done when

The smoke test above is run for real and its actual output is pasted into this file, and
`PLAN.md` / `README.md` / `CHANGELOG.md` are updated in the same commit.
