# Architecture

## System Overview

`doraemon-box` is a personal CMS platform for tracking experiences and knowledge items (books, manga, articles, animation, movies, series, notes, links).

The system consists of:

- Backend API server (Rust/Axum) - `dokodemo-door` crate
- Separate frontend client - `takekoputaa` (planned)
- Telegram bot integration for chat-based capture

PostgreSQL is the primary persistence layer with SQLite as local dev fallback.

## Project Structure

```text
.
├── Cargo.toml                       # workspace manifest
├── AGENTS.md                        # agent briefing
├── CLAUDE.md                        # Claude Code config
├── .agents/                         # shared agent state/config
├── .github/workflows/ci.yml         # GitHub Actions CI
├── docs/                            # project documentation
├── deploy/
│   └── k8s/                         # k3s deployment manifests
│       ├── kustomization.yaml
│       ├── namespace.yaml
│       ├── configmap.yaml
│       ├── deployment.yaml
│       ├── service.yaml
│       └── secret.example.yaml
├── dokodemo-door/                   # backend crate
│   ├── Cargo.toml
│   ├── config.yaml
│   ├── migrations/
│   │   ├── postgres/0001_init.sql   # PostgreSQL schema
│   │   └── sqlite/0001_init.sql     # SQLite schema
│   ├── sql/dokodemo-schema.sql      # schema notes
│   └── src/
│       ├── main.rs                  # server, routes, handlers, DB logic
│       ├── lib.rs                   # library root
│       └── db/                      # repository trait abstraction
│           ├── mod.rs               # Repository trait definition
│           ├── postgres.rs          # PostgreSQL impl stub
│           └── sqlite.rs            # SQLite impl stub
└── takekoputaa/                     # future frontend
    └── README.md
```

## Module Map

Current (all in `dokodemo-door/src/main.rs`):

- Server bootstrap: tracing, DB connection, migration, Axum router
- API routes: CRUD entries, quick-capture, Telegram webhook
- Auth middleware: Bearer/x-api-key validation
- DB layer: dual PostgreSQL/SQLite via sqlx with inline queries
- Validation: kind and status enum checks
- Helpers: URL extraction, title summarization, tag serialization

Scaffolded but unused:

- `db/mod.rs`: `Repository` trait
- `db/postgres.rs`: `PostgresRepository` stub
- `db/sqlite.rs`: `SqliteRepository` stub

Planned modules (future refactor):

- `domain`: content item models, tags, categories
- `application`: use-cases/services
- `api`: HTTP handlers, request/response schemas
- `infra`: repository implementations, migrations

## Data Flow

1. Client sends request to `/api/v1/*` with auth header.
2. `api_key_auth` middleware validates credentials.
3. Handler validates input (kind, status enums).
4. DB function executes sqlx query against PostgreSQL or SQLite.
5. Response returns JSON entry with tags, timestamps, metadata.

## Dependencies

Backend crate key dependencies:

- `axum` 0.6: HTTP routing and handlers
- `tokio` 1.33: async runtime
- `tower`, `tower-http`, `tower-layer`: middleware (compression, tracing, auth)
- `sqlx` 0.7: database access (postgres, sqlite, migrate, chrono, uuid)
- `serde`, `serde_json`: JSON serialization
- `tracing`, `tracing-subscriber`: structured logging
- `anyhow`, `thiserror`: error handling
- `chrono`, `uuid`: timestamps and identifiers
