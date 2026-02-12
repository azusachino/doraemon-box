# Project Design

## Design Goals

- Fast personal capture of experiences and useful references
- Clean separation between backend API and frontend
- Extensible ingestion channels (web/app first, Telegram bot early)
- Maintainable Rust codebase with clear module boundaries

## Key Decisions

| Decision         | Choice                  | Rationale                                                           |
| ---------------- | ----------------------- | ------------------------------------------------------------------- |
| Language         | Rust                    | Strong correctness, performance, and ecosystem fit for API services |
| Build System     | Cargo                   | Native Rust build/test/tooling standard                             |
| API Framework    | Axum                    | Ergonomic async web framework integrated with Tower ecosystem       |
| Persistence      | PostgreSQL              | Reliable relational storage for structured content + tags           |
| App Shape        | API + separate frontend | Clean API contract and independent UI evolution                     |
| Chat Integration | Telegram bot            | Frictionless quick capture from daily chat workflows                |

## Trade-offs

- API-first adds upfront contract design cost but improves long-term extensibility.
- PostgreSQL introduces setup overhead versus file/local-only storage, but gives better querying, integrity, and migration workflows.
- Telegram bot integration adds surface area (auth, parsing, webhook/polling) but significantly improves capture speed.
