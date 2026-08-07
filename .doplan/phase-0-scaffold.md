# Phase 0 — Scaffold ✅

Goal: a repo that builds, runs, and carries the research the design rests on.

## Tasks

- [x] Cargo project, edition 2024, crate `cust-code` / binary `cust`
- [x] `cust help`, `cust -h`, `cust --help` print usage; no args does the same
- [x] `cust version`, `-V`, `--version` print the crate version
- [x] Unknown command exits non-zero and names the command
- [x] `.gitignore`, MIT license declared in `Cargo.toml`
- [x] Release profile: `lto`, `codegen-units = 1`, `strip`
- [x] Read all nine reference agents and write `.knowledge/` + `DESIGN-NOTES.md`
- [x] `PLAN.md` with a smoke test per phase
- [x] `tests/cli.rs` — integration tests that run the real binary
- [x] `.learning/`, `.memory/`, `.doplan/` established

## Smoke test — run 2026-08-08, passed

```
$ cargo fmt --check                              → fmt OK
$ cargo clippy --all-targets -- -D warnings      → clippy OK
$ cargo test                                     → 4 passed; 0 failed

$ cust
cust — a coding agent CLI

usage: cust <command>

  help      show this message
  version   print the version

$ cust version
0.0.0

$ cust bogus
unknown command: bogus
exit=1
```

## Notes

- No model call exists yet. The binary is a shell in both senses.
- `LF will be replaced by CRLF` warnings on every commit (Windows). Harmless, but add a
  `.gitattributes` with `* text=auto eol=lf` in Phase 1 to silence it.
