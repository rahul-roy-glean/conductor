# Conductor

Orchestrate multiple Claude Code agents in parallel from a single workspace. Define goals, decompose them into tasks with AI, dispatch agents across isolated git worktrees, and monitor everything through a chat-first web UI.

![CI](https://github.com/rahul-roy-glean/conductor/actions/workflows/ci.yml/badge.svg)

## Screenshots

### Project Sidebar & Chat
![Chat View](.github/screenshots/dashboard.png)

### Fleet View
![Fleet View](.github/screenshots/goals.png)

## How It Works

1. **Describe a goal** in plain language, pointed at a repo
2. **Chat about it** — Conductor streams an AI conversation to help you plan
3. **Decompose** — AI breaks the goal into ordered tasks with dependencies
4. **Review & approve** — edit the proposed task list inline before creating
5. **Dispatch** — each task gets its own Claude Code agent in an isolated git worktree
6. **Monitor** — watch agents work in real-time via structured event streams
7. **Merge** — completed work lands on branches ready for review

## Installation

### Homebrew (macOS)

```sh
brew tap rahul-roy-glean/devtools
brew install conductor
```

### From Source

Requires Rust 1.83+ and Node.js 18+.

```sh
git clone https://github.com/rahul-roy-glean/conductor.git
cd conductor
make build
make install  # copies to /usr/local/bin
```

Or manually:

```sh
cd frontend && npm ci && npm run build && cd ..
cargo build --release
cp target/release/conductor /usr/local/bin/
```

### Prerequisites

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installed and authenticated:
  ```sh
  npm install -g @anthropic-ai/claude-code
  claude  # authenticate on first run
  ```
- Git (for worktree-based isolation)

## Quick Start

```sh
# Start the server (UI at http://localhost:3001)
conductor server

# Or start and open browser
conductor ui
```

### Web UI

1. **Add a project** — click "+ Add Project" in the sidebar, enter a repo path
2. **Create a goal** — click "+" next to a project, describe what you want
3. **Chat** — the Chat tab opens by default. Ask questions, request decomposition
4. **Decompose** — click "Decompose" in the Tasks tab, or ask in chat. Review the proposed tasks inline
5. **Dispatch** — click "Dispatch All" or approve a task proposal to spawn agents
6. **Monitor** — watch agents in the Agents tab or Fleet view

**Shortcuts:** `Cmd+K` command palette, `Cmd+N` new goal, `Esc` close dialogs

### CLI

```sh
conductor goal create "Add user authentication" --repo /path/to/project
conductor goal decompose <goal-id>
conductor goal dispatch <goal-id>
conductor status
conductor logs <agent-id>
conductor nudge <agent-id> "Focus on the middleware first"
conductor kill <agent-id>
conductor cleanup
```

## Documentation

- [API Reference](docs/api.md) — all REST endpoints, SSE streams, settings
- [Architecture](docs/architecture.md) — system design, modules, project structure
- [Development](docs/development.md) — local setup, testing, CI, releasing

## License

MIT
