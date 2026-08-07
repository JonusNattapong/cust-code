# cust-code

A coding agent CLI. The binary is `cust`.

## Status

Pre-alpha — Phase 0 of [PLAN.md](PLAN.md). The binary builds and prints help; it does not
talk to a model yet.

| Doc | What it is |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | The design — core contracts, crate map, and what was taken from whom |
| [PLAN.md](PLAN.md) | Phased build order, with a smoke test per phase |
| [DESIGN-NOTES.md](DESIGN-NOTES.md) | Condensed survey of nine coding agents — the design rests on this |
| [.knowledge/](.knowledge/00-index.md) | The survey in long form, split by subsystem |
| [.doplan/](.doplan/README.md) | What is being worked on right now |
| [.learning/](.learning/LEARNINGS.md) | Lessons and errors from building this |
| [.memory/](.memory/MEMORY.md) | Durable facts, decisions, and working preferences |
| [CHANGELOG.md](CHANGELOG.md) | What changed |

The four dot-directories divide cleanly: `.knowledge/` is what we learned from *other*
projects before writing code, `.learning/` is what we learn from *our own* while writing it,
`.memory/` is what a fresh session needs to know, and `.doplan/` is the current task list.

## Development

```bash
cargo build
cargo run -- help
```

Gate before committing:

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

## License

MIT
