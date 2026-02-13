# AGENTS

## Project Overview

`doraemon-box` is a personal CMS for tracking life experiences and useful knowledge:

- Reading: books, manga, articles
- Watching: animation, movies, series
- Quick notes
- Useful links

The platform consists of:

- Backend API server (Rust/Axum) - **implemented** as `dokodemo-door`
- Separate frontend client - planned as `takekoputaa`
- Telegram bot integration for fast capture - **webhook endpoint implemented**

## Architecture

Style: API server + separate frontend, PostgreSQL primary with SQLite fallback.

Current code structure:

```text
.
├── Cargo.toml                       # workspace manifest
├── AGENTS.md                        # project briefing for agents
├── CLAUDE.md                        # Claude Code config
├── .agents/                         # shared agent state/config
├── docs/                            # project documentation
├── deploy/k8s/                      # k3s deployment manifests
├── dokodemo-door/                   # backend crate
│   ├── Cargo.toml
│   ├── config.yaml
│   ├── migrations/
│   │   ├── postgres/0001_init.sql
│   │   └── sqlite/0001_init.sql
│   ├── sql/dokodemo-schema.sql      # schema notes
│   └── src/
│       ├── main.rs                  # server, routes, handlers, DB logic
│       ├── lib.rs                   # library root (re-exports db module)
│       └── db/                      # repository trait abstraction
│           ├── mod.rs
│           ├── postgres.rs
│           └── sqlite.rs
└── takekoputaa/                     # future frontend
```

Data flow:

1. Client (frontend or Telegram bot) sends request to `/api/v1/*`.
2. `api_key_auth` middleware validates Bearer token or `x-api-key` header.
3. Handler validates input, constructs domain entry.
4. `insert_entry` / query functions interact with PostgreSQL or SQLite via sqlx.
5. Response returns JSON entry with tags, timestamps, and metadata.

## Build & Run

Task runner: `mise` (see `mise.toml`).

| Command              | Description                       |
| -------------------- | --------------------------------- |
| `mise run fmt`       | Format all (cargo fmt + prettier) |
| `mise run fmt-check` | Check formatting                  |
| `mise run lint`      | Clippy with -D warnings           |
| `mise run test`      | Run all tests                     |
| `mise run check`     | fmt-check + lint + test           |
| `mise run migrate`   | Run DB migrations only            |

Direct commands:

- Build: `cargo build --workspace`
- Run: `cargo run -p dokodemo-door`
- Run (PG): `DATABASE_URL='postgres://...' cargo run -p dokodemo-door`

## Conventions

- Naming: `snake_case` for functions/modules/files, `PascalCase` for types
- Error handling: `Result`-based with `thiserror` for domain errors, `anyhow` at boundaries
- API: versioned under `/api/v1`, JSON payloads
- Auth: `APP_API_KEY` env var, Bearer or x-api-key header
- Async: structured async with clear ownership via Tokio
- Testing: unit tests inline, integration tests for handlers/repositories
- Commits: conventional style, no emojis
- Formatting: rustfmt (Rust), prettier (md/json/yaml)

## Key Files

- `Cargo.toml` - workspace manifest
- `dokodemo-door/Cargo.toml` - backend crate deps
- `dokodemo-door/src/main.rs` - entrypoint with all handlers and DB logic
- `dokodemo-door/migrations/` - sqlx migrations (postgres + sqlite)
- `deploy/k8s/` - k3s manifests (deployment, service, configmap, secret)
- `mise.toml` - task runner config
- `rustfmt.toml` - Rust formatting rules
- `clippy.toml` - Clippy thresholds

## Quality Standards

Required before merge:

- `mise run check` (fmt-check + lint + test)

Pre-commit hook auto-formats on commit.

Additional expectations:

- New features include tests
- No warnings introduced
- Docs stay aligned with code changes
