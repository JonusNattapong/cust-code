# Changelog

All notable changes to this project are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Cargo scaffold: crate `cust-code`, binary `cust`, with `help` and `version` commands.
- Integration tests in `tests/cli.rs` that run the real binary, so PLAN.md's smoke checks
  are enforced rather than remembered.
- `.knowledge/` — the long-form study of nine coding agents (codex, grok-build, kimi-cli,
  mistral-vibe, hermes-agent, MiMo-Code, openclaw, prime-agent, clew-code), split by
  subsystem: code mode, daemon/sessions, compaction, sandboxing, tools/events, subagents,
  skills/extensions, boundaries, providers, protocols.
- `DESIGN-NOTES.md` — condensed version of the above.
- `PLAN.md` — phased build order with a smoke test per phase.
