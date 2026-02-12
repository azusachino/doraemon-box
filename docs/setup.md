# Setup

## Prerequisites

- Rust toolchain (stable) with Cargo
- `rustfmt` component
- `clippy` component
- SQLite3 (default local test mode)
- PostgreSQL (production mode)

Optional:

- `mise` (task/environment management, if adopted later)

## Installation

```bash
git clone <repo-url>
cd doraemon-box
cargo fetch
```

## Build

```bash
cargo build --workspace
```

## Run

Current backend prototype:

```bash
# Local default (SQLite)
cargo run -p dokodemo-door
```

Server binds to `127.0.0.1:3000` by default.

Run with PostgreSQL:

```bash
export DATABASE_URL='postgres://user:password@localhost:5432/doraemon_box'
cargo run -p dokodemo-door
```

SQLite local test database default:

```text
sqlite://./data/doraemon-box.db?mode=rwc
```

Enable API auth (recommended for non-local usage):

```bash
export APP_API_KEY='replace-with-long-random-key'
```

Authenticated request example:

```bash
curl -H "Authorization: Bearer $APP_API_KEY" http://127.0.0.1:3000/api/v1/entries
```

## Migrations

Run migrations only (without starting the HTTP server):

```bash
cargo run -p dokodemo-door -- --migrate-only
```

Or via mise:

```bash
mise run migrate
```

## Test

```bash
cargo test --workspace --all-features
```

## Quality Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## k3s Deployment

Edit `deploy/k8s/secret.example.yaml` with real values, then:

```bash
kubectl apply -k deploy/k8s
```

The deployment runs migrations first through an `initContainer` using `--migrate-only`.
