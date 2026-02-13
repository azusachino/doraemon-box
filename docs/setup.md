# Setup

## Prerequisites

- [mise](https://mise.jdx.dev/) (manages Rust toolchain and npm tools)
- PostgreSQL (production) or SQLite3 (local dev)

mise will install:

- Rust toolchain (stable) with rustfmt and clippy
- prettier (markdown/JSON/YAML formatting)
- markdownlint-cli2 (markdown linting)

## Installation

```bash
git clone <repo-url>
cd doraemon-box
mise install
cargo fetch
```

## Build

```bash
cargo build --workspace
```

## Run

Local default (SQLite):

```bash
cargo run -p dokodemo-door
```

Server binds to `127.0.0.1:3000`.

With PostgreSQL:

```bash
export DATABASE_URL='postgres://user:password@localhost:5432/doraemon_box'
cargo run -p dokodemo-door
```

With API auth (recommended for non-local):

```bash
export APP_API_KEY='replace-with-long-random-key'
```

## Migrations

Run migrations only (no HTTP server):

```bash
mise run migrate
# or: cargo run -p dokodemo-door -- --migrate-only
```

## Test

```bash
mise run test
# or: cargo test --workspace --all-features
```

## Quality Checks

```bash
mise run check    # runs fmt-check + lint + test
mise run fmt      # auto-format all files
mise run lint     # clippy with -D warnings
```

## k3s Deployment

Copy and edit `deploy/k8s/secret.example.yaml` with real values, then:

```bash
kubectl apply -k deploy/k8s
```

The deployment runs migrations via an `initContainer` using `--migrate-only`.
