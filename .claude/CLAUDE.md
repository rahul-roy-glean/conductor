# Conductor Project Conventions

## Architecture
- Rust backend (axum + tokio) with SQLite database
- React frontend (Vite + TypeScript + Tailwind)
- Orchestrates Claude Code headless sessions via `claude -p --output-format stream-json`

## Code Style
- Use `anyhow::Result` for application-level errors
- Use `thiserror` for library-level error types
- Prefer `Arc<RwLock<T>>` for shared mutable state across async tasks
- Use axum extractors (State, Path, Json) for handler signatures
- All database operations go through `db::queries`
- UUIDs stored as TEXT in SQLite
- Timestamps in ISO 8601 format

## Module Organization
- `src/agent/` - Claude Code process lifecycle
- `src/server/` - HTTP API and SSE
- `src/goal/` - Goal spaces and task management
- `src/db/` - SQLite schema and queries
- `src/hooks/` - Claude Code hooks integration
- `src/cli.rs` - CLI command definitions

## Testing
- Unit tests in each module
- Integration tests in `tests/`
- Use `cargo test` to run all tests
