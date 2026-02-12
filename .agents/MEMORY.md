# Agent Memory

## Personal Defaults

- Use mise for task running when available
- Prefer language-native formatters (zig fmt, rustfmt, gofmt, prettier)
- No emojis in git commit messages
- Prose wrap at 80 characters
- 2-space indentation for config files (JSON, YAML, TOML)
- Always ask before committing or pushing
- Format and lint before every commit
- Run tests before creating PRs/MRs
- Keep solutions simple - avoid over-engineering

## Project Patterns

- Rust workspace with a backend crate: `dokodemo-door`
- Backend stack: Axum + Tokio + Tower + Tracing
- Current direction: personal CMS with separate frontend
- Primary DB target: PostgreSQL
- Additional ingestion target: Telegram bot integration
- Existing DB abstraction scaffolding in `dokodemo-door/src/db/`
- SQL assets under `dokodemo-door/sql/`
- Docs live under `docs/`

## Decisions

- Architecture style: API server + separate frontend
- Storage: prefer PostgreSQL
- Integrations: Telegram bot support for chat-based quick capture
- Conventions: mature Rust best practices
- Required quality gates:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features`

## Debugging Notes

