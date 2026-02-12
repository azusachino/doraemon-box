# Architecture

## System Overview

`doraemon-box` is a personal CMS platform for tracking experiences and knowledge items (books, manga, articles, animation, movies, series, notes, links).

The target system is split into:

- Backend API server (Rust/Axum)
- Separate frontend client
- Telegram bot integration for chat-based capture

PostgreSQL is the primary persistence layer.

## Project Structure

```text
.
├── Cargo.toml                  # workspace manifest
├── AGENTS.md                   # project briefing for coding agents
├── .agents/                    # shared agent state/config
├── docs/                       # project documentation
├── dokodemo-door/              # backend crate
│   ├── Cargo.toml
│   ├── config.yaml
│   ├── sql/
│   │   └── dokodemo-schema.sql
│   └── src/
│       ├── main.rs             # current API server prototype
│       ├── lib.rs
│       └── db/
│           ├── mod.rs
│           ├── postgres.rs
│           └── sqlite.rs
└── takekoputaa/                # additional module/docs placeholder
```

## Module Map

- `dokodemo-door/src/main.rs`: Axum server bootstrap, middleware, routes, and runtime wiring.
- `dokodemo-door/src/db/mod.rs`: repository abstraction entry point.
- `dokodemo-door/src/db/postgres.rs`: PostgreSQL repository implementation placeholder.
- `dokodemo-door/src/db/sqlite.rs`: SQLite repository placeholder (not primary target).
- `dokodemo-door/sql/`: schema and SQL assets.

Planned modules (next iteration):

- `domain`: content item models, tags, categories, states
- `application`: use-cases/services for create/update/query
- `api`: HTTP handlers, request/response schemas, versioned routes `/api/v1`
- `infra`: PostgreSQL repositories, migrations, external integrations
- `integrations/telegram`: bot command handlers and message parsing

## Data Flow

1. User captures content via frontend form or Telegram bot.
2. Backend API validates and normalizes payload.
3. Application layer maps payload to domain entities.
4. Repository persists entities and relations in PostgreSQL.
5. API returns structured resources for timeline, filters, and search.

## Dependencies

Current key dependencies in backend crate:

- `axum`: HTTP routing and handler framework
- `tokio`: async runtime
- `tower`, `tower-http`, `tower-layer`: middleware and service composition
- `sqlx`: database access layer
- `serde`, `serde_json`: serialization/deserialization
- `tracing`, `tracing-subscriber`: structured logging
- `anyhow`, `thiserror`: error handling
