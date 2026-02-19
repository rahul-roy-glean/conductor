# Development

## Setup

```sh
git clone https://github.com/rahul-roy-glean/conductor.git
cd conductor

# Install git pre-commit hook (runs lint + format checks)
make setup-hooks
```

## Running Locally

```sh
# Run frontend dev server (hot reload on :5173) + backend (:3001) concurrently
make dev
```

Or run separately:

```sh
# Terminal 1: backend
cargo run -- server

# Terminal 2: frontend with hot reload
cd frontend && npm run dev
```

The Vite dev server proxies `/api` requests to the backend at `localhost:3001`.

## Commands

```sh
make build        # Build frontend (npm ci + vite build) then cargo build --release
make test         # cargo test + vitest
make lint         # cargo clippy + eslint
make check        # cargo fmt --check + prettier --check
make fmt          # Auto-format everything
make clean        # Remove build artifacts
make install      # Build and copy binary to /usr/local/bin
```

## CI

The CI pipeline runs 8 parallel jobs on every push and PR:

| Job | Check |
|-----|-------|
| Rustfmt | `cargo fmt -- --check` |
| Clippy | `cargo clippy -- -D warnings` |
| Tests | `cargo test` |
| MSRV | `cargo check` with Rust 1.83 |
| Cargo Deny | License audit, security advisories |
| Frontend Build | TypeScript + Vite build |
| Frontend Lint | Prettier + ESLint |
| Frontend Tests | Vitest |

## Releasing

Pushing a version tag triggers cross-platform binary builds:

```sh
# Update version in Cargo.toml, then:
make release  # tags vX.Y.Z from Cargo.toml and pushes
```

Builds for:
- macOS ARM (aarch64-apple-darwin)
- macOS Intel (x86_64-apple-darwin)
- Linux x86_64 (x86_64-unknown-linux-gnu)

## Running as a Service

```sh
# With Homebrew
brew services start conductor

# Manually
conductor server --port 3001
```
