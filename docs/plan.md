# Implementation Plan

## Current Phase

Tag & category management: normalize tags and categories into proper tables with CRUD endpoints.

## Roadmap

- [ ] Refactor `main.rs` into domain/application/api/infra modules
- [ ] Add integration tests for API endpoints
- [x] Implement tag and category management endpoints
- [ ] Add search/filter improvements (full-text, date range)
- [ ] Build `takekoputaa` frontend client
- [ ] Configure Telegram bot polling/webhook setup automation

## Tag & Category Management (Phase 2)

### Schema

- `categories` table (id, name UNIQUE, description, created_at)
- `tags` table (id, name UNIQUE, created_at)
- `entry_tags` junction table (entry_id FK, tag_id FK)
- Remove CHECK constraint on `entries.kind`; validate against `categories` table
- Seed existing 8 kinds into `categories`
- Migrate `tags_json` data into `tags` + `entry_tags`

### Endpoints

- `GET/POST /api/v1/categories`, `PATCH/DELETE /api/v1/categories/:id`
- `GET/POST /api/v1/tags`, `DELETE /api/v1/tags/:id`
- Add `tag` query param to `GET /api/v1/entries`

### Design Decisions

- Normalized schema over JSON blobs
- Async `validate_kind` queries DB instead of hardcoded constant
- `tags_json` column retained during transition
- Entry API response shape unchanged (`tags: Vec<String>`, `kind: String`)
- Fixed seed UUIDs for deterministic category seeding

## Completed

- [x] Agent infrastructure initialized (AGENTS.md, .agents/, CLAUDE.md)
- [x] Project documentation scaffolded
- [x] Base architecture captured (API + frontend + Telegram, PG + SQLite)
- [x] CRUD API endpoints (`/api/v1/entries`)
- [x] Quick-capture endpoint
- [x] Telegram webhook endpoint
- [x] API key authentication middleware
- [x] Dual database support (PostgreSQL + SQLite) with migrations
- [x] k3s deployment manifests
- [x] CI pipeline (GitHub Actions with mise)
- [x] Pre-commit hook for auto-formatting
