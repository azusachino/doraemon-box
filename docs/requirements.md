# Requirements

## Functional Requirements

- [ ] Capture content entries for:
  - books, manga, articles
  - animation, movies, series
  - quick notes
  - useful links
- [ ] Support CRUD operations for entries through API.
- [ ] Support categorization and tagging.
- [ ] Support status tracking (e.g., planned, in-progress, completed, dropped).
- [ ] Provide timeline/list views with filtering and sorting.
- [ ] Allow quick entry creation from Telegram bot chat commands/messages.
- [ ] Expose versioned API endpoints under `/api/v1`.

## Non-Functional Requirements

- Performance: low-latency CRUD and list queries for personal-scale dataset.
- Security: authentication/authorization for API and Telegram bot integration.
- Reliability: durable storage in PostgreSQL with recoverable migrations.

## Constraints

- Backend language/framework: Rust + Axum.
- Architecture: API server with separate frontend client.
- Database preference: PostgreSQL.
- Quality gates before merge:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features`
