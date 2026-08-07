# cust-code

A coding agent CLI. The binary is `cust`.

## Status

All phases (Phase 0 through Phase 34) complete — [PLAN.md](PLAN.md). `cust` is a fully featured coding agent supporting tool calling, process sandboxing with self-protection and Custbox isolation, QuickJS code mode, session storage with leases and rewind, middle-region trajectory compaction with parallel batch compactor, provider capabilities system and failover group, rich Ink-style component TUI engine with live memory budget meter and slash command autocomplete, ACP protocol daemon server, subagents with programmatic REPL invocations, budgeted skill discovery (`SKILL.md`), system reminders, fast in-process git tracking, debounced file watching, trajectory refinement, PTY interactive terminal execution, MCP client connector, atomic model runtime generations, and first-time onboarding wizard.

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

## Usage

```bash
cust "read Cargo.toml and tell me the edition"
cust -y "write hello to target/output.txt"
cust ask "reply with the single word: ok"
```

## Development

```bash
cargo build --workspace
cargo run --bin cust -- help
```

Gate before committing:

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

## License

MIT
