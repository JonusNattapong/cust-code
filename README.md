# cust-code

A coding agent CLI. The binary is `cust`.

## Status

Pre-alpha — Phase 0 of [PLAN.md](PLAN.md). The binary builds and prints help; it does not
talk to a model yet.

| Doc | What it is |
|---|---|
| [PLAN.md](PLAN.md) | Phased build order, with a smoke test per phase |
| [DESIGN-NOTES.md](DESIGN-NOTES.md) | Condensed survey of nine coding agents — the design rests on this |
| [.knowledge/](.knowledge/00-index.md) | The long form, split by subsystem |
| [CHANGELOG.md](CHANGELOG.md) | What changed |

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
