# Feature Status

| Feature                     | Status      | Owner | Notes                                        |
| --------------------------- | ----------- | ----- | -------------------------------------------- |
| Content entry CRUD API      | done        | -     | POST/GET/PATCH/DELETE `/api/v1/entries`      |
| Quick-capture endpoint      | done        | -     | POST `/api/v1/quick-capture`                 |
| Telegram webhook capture    | done        | -     | POST `/api/v1/integrations/telegram/update`  |
| API key authentication      | done        | -     | Bearer token and x-api-key header            |
| Database migrations         | done        | -     | PostgreSQL + SQLite via sqlx                 |
| k3s deployment              | done        | -     | kustomize manifests with initContainer       |
| CI pipeline                 | done        | -     | GitHub Actions with `mise run check`         |
| Category and tag management | in-progress | -     | normalized tables + CRUD endpoints           |
| Timeline/filter/search      | in-progress | -     | basic kind/status/search filters implemented |
| Code modularization         | planned     | -     | refactor main.rs into modules                |
| Frontend client             | planned     | -     | `takekoputaa` not started                    |

## Status Legend

- **planned**: not started
- **in-progress**: actively being worked on
- **review**: implementation complete, under review
- **done**: merged and verified
- **blocked**: cannot proceed, see notes
