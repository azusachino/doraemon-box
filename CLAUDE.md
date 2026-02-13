# CLAUDE.md

## Project

Personal CMS backend (`doraemon-box`) - Rust/Axum API server with PostgreSQL + SQLite dual support.

## Build & Quality

```bash
mise run fmt        # format all (cargo fmt + prettier)
mise run lint       # cargo clippy -D warnings
mise run test       # cargo test --workspace
mise run check      # fmt-check + lint + test
mise run migrate    # run DB migrations only
```

## Conventions

- Rust naming: `snake_case` functions/modules, `PascalCase` types
- Error handling: `thiserror` for domain errors, `anyhow` at app boundaries
- API: versioned under `/api/v1`, JSON request/response
- Auth: Bearer token via `APP_API_KEY` env var
- Commits: conventional style, no emojis
- Config files: 2-space indent (JSON, YAML, TOML)
- Rust files: 4-space indent, max 100 columns

## Key Paths

- `dokodemo-door/` - backend crate (Axum server)
- `dokodemo-door/src/main.rs` - entrypoint, routes, handlers, DB logic
- `dokodemo-door/migrations/` - sqlx migrations (postgres/ and sqlite/)
- `deploy/k8s/` - k3s deployment manifests
- `takekoputaa/` - future frontend placeholder

## Before Committing

- Run `mise run check` (or at minimum `mise run fmt`)
- Never commit `.env` files or secrets
