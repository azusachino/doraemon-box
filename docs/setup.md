# Setup

## Prerequisites

- Rust toolchain (stable) with Cargo
- `rustfmt` component
- `clippy` component
- PostgreSQL (for target architecture)

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
cargo run -p dokodemo-door
```

Server currently binds to `127.0.0.1:3000`.

## Test

```bash
cargo test --workspace --all-features
```

## Quality Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
