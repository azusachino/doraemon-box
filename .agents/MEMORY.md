# Agent Memory

## Personal Defaults

- Use mise for task running when available
- Prefer language-native formatters (rustfmt, prettier)
- No emojis in git commit messages
- Prose wrap at 80 characters
- 2-space indentation for config files (JSON, YAML, TOML)
- Always ask before committing or pushing
- Format and lint before every commit
- Run tests before creating PRs/MRs
- Keep solutions simple - avoid over-engineering

## Project Patterns

- Rust workspace with backend crate `dokodemo-door`
- Backend stack: Axum 0.6 + Tokio + Tower + Tracing
- Dual DB support: PostgreSQL (primary) + SQLite (local dev fallback)
- sqlx migrations in `dokodemo-door/migrations/{postgres,sqlite}/`
- All handlers and DB logic currently live in `main.rs` (monolith file)
- Repository trait in `db/mod.rs` is scaffolded but unused by main.rs
- `takekoputaa/` is a future frontend placeholder
- k3s deployment in `deploy/k8s/` with initContainer for migrations
- Pre-commit hook runs `mise run fmt` and re-stages files
- CI uses `mise run check` via GitHub Actions

## API Shape

- `GET /health` - health check (no auth)
- `POST /api/v1/entries` - create entry (auth required)
- `GET /api/v1/entries` - list entries with filters (auth required)
- `GET /api/v1/entries/:id` - get single entry (auth required)
- `PATCH /api/v1/entries/:id` - update entry (auth required)
- `DELETE /api/v1/entries/:id` - delete entry (auth required)
- `POST /api/v1/quick-capture` - quick note capture (auth required)
- `POST /api/v1/integrations/telegram/update` - Telegram webhook (secret token auth)
- `GET/POST /api/v1/categories` - list/create categories (auth required)
- `PATCH/DELETE /api/v1/categories/:id` - update/delete categories (auth required)
- `GET/POST /api/v1/tags` - list/create tags (auth required)
- `DELETE /api/v1/tags/:id` - delete tag (auth required)

## Decisions

- Architecture: API server + separate frontend
- Storage: PostgreSQL primary, SQLite for local dev
- Integrations: Telegram bot for chat-based capture
- Auth: Bearer token or x-api-key header via `APP_API_KEY` env var
- Conventional commits, no emojis
- Quality gates: fmt-check + clippy + test
- Tags/categories: normalized schema with junction table, not JSON blobs
- validate_kind is async (queries categories table) not hardcoded
- tags_json column retained during transition for backward compat
- Fixed seed UUIDs for deterministic category seeding across DB backends

## Debugging Notes

