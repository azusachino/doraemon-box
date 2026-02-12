# AGENTS

## Project Overview

`doraemon-box` is a personal CMS for tracking life experiences and useful knowledge:

- Reading: books, manga, articles
- Watching: animation, movies, series
- Quick notes
- Useful links

The target product shape is a backend-first platform with:

- API server
- Separate frontend client
- Telegram bot integration for fast capture in chat

Current repository state includes an early Rust backend crate (`dokodemo-door`) with Axum/Tokio dependencies and placeholder DB modules.

## Architecture

Planned style:

- API server + separate frontend
- PostgreSQL as primary database
- Telegram bot as an additional ingestion interface

Current code structure:

- Root workspace with `dokodemo-door` member
- `dokodemo-door/src/main.rs`: Axum server prototype with middleware and routes
- `dokodemo-door/src/db/`: repository abstraction placeholders (`postgres`, `sqlite`)
- `dokodemo-door/sql/`: SQL schema placeholders
- `docs/`: early docs placeholders

Likely high-level flow (target):

- Frontend and Telegram bot send content capture requests to backend API
- Backend validates, normalizes, and persists records to PostgreSQL
- Backend exposes query/filter endpoints for timeline, tags, categories, and search

## Build & Run

Workspace commands:

- Build: `cargo build --workspace`
- Run backend (current): `cargo run -p dokodemo-door`
- Test: `cargo test --workspace --all-features`

Quality checks:

- Format: `cargo fmt --all --check`
- Lint: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## Conventions

Use mature Rust best practices:

- Naming: `snake_case` for functions/modules/files, `PascalCase` for types
- Error handling: `Result`-based flow with `thiserror` for domain errors and `anyhow` at app boundaries
- API conventions: versioned endpoints under `/api/v1`
- Async/concurrency: prefer structured async with clear ownership and minimal lock scope
- Testing: unit tests for pure/domain logic + integration tests for handlers/repositories

## Key Files

- `Cargo.toml`: workspace manifest
- `Cargo.lock`: dependency lockfile
- `dokodemo-door/Cargo.toml`: backend crate dependencies/config
- `dokodemo-door/src/main.rs`: current Axum entrypoint
- `dokodemo-door/src/lib.rs`: backend library root
- `dokodemo-door/src/db/mod.rs`: repository trait placeholder
- `dokodemo-door/src/db/postgres.rs`: PostgreSQL repository placeholder
- `dokodemo-door/src/db/sqlite.rs`: SQLite repository placeholder
- `dokodemo-door/sql/dokodemo-schema.sql`: schema notes placeholder
- `docs/README.md`: documentation index placeholder

## Quality Standards

Required before merge:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`

Additional expectations:

- New features include tests
- Avoid introducing warnings
- Keep docs aligned with architecture and command changes
