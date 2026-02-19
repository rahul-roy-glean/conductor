# Conductor

Orchestrate multiple Claude Code agents in parallel from a single workspace. Define goals, decompose them into tasks with AI, dispatch agents across isolated git worktrees, and monitor everything through a chat-first web UI.

![CI](https://github.com/rahul-roy-glean/conductor/actions/workflows/ci.yml/badge.svg)

## What It Does

1. **Describe a goal** in plain language, pointed at a repo
2. **Chat about it** — Conductor streams an AI conversation to help you plan
3. **Decompose** — AI breaks the goal into ordered tasks with dependencies
4. **Review & approve** — edit the proposed task list inline before creating
5. **Dispatch** — each task gets its own Claude Code agent in an isolated git worktree
6. **Monitor** — watch agents work in real-time via structured event streams
7. **Merge** — completed work lands on branches ready for review

Each agent runs as a headless `claude -p --output-format stream-json` process. Conductor parses the NDJSON event stream, tracks tool usage, costs, and status, and auto-dispatches dependent tasks as predecessors complete.

## Screenshots

### Project Sidebar & Chat
Left sidebar with project hierarchy. Chat tab is the default goal view — describe goals conversationally, get streamed AI responses with inline task proposals.

![Chat View](.github/screenshots/dashboard.png)

### Fleet View
Real-time overview of all agents across repositories, grouped by project with status, cost tracking, and elapsed time.

![Fleet View](.github/screenshots/goals.png)

## Installation

### Homebrew (macOS)

```sh
brew tap rahul-roy-glean/devtools
brew install conductor
```

### From Source

**Requirements**: Rust 1.83+, Node.js 18+

```sh
git clone https://github.com/rahul-roy-glean/conductor.git
cd conductor

# Build frontend and backend
make build

# Install to /usr/local/bin
make install
```

Or manually:

```sh
# Build frontend
cd frontend && npm ci && npm run build && cd ..

# Build backend (embeds frontend assets)
cargo build --release

# Copy binary
cp target/release/conductor /usr/local/bin/
```

### From Cargo

```sh
# Clone and install directly
cargo install --path .
```

Note: you need to build the frontend first (`cd frontend && npm ci && npm run build`) before `cargo install`, since the binary embeds the frontend assets at compile time.

### Prerequisites

Conductor orchestrates [Claude Code](https://docs.anthropic.com/en/docs/claude-code) agents, so you need:

- **Claude Code CLI** installed and authenticated:
  ```sh
  npm install -g @anthropic-ai/claude-code
  claude  # authenticate on first run
  ```
- **Git** (for worktree-based isolation)

## Quick Start

```sh
# Start the server with embedded UI
conductor server
# Open http://localhost:3001

# Or start and auto-open in browser
conductor ui
```

### Using the Web UI

1. **Add a project** — click "+ Add Project" in the sidebar, enter a repo path
2. **Create a goal** — click "+" next to a project, describe what you want to accomplish
3. **Chat** — the Chat tab opens by default. Ask questions, request decomposition
4. **Decompose** — switch to Tasks tab and click "Decompose", or ask in chat. Review the proposed tasks inline
5. **Dispatch** — click "Dispatch All" or approve a task proposal to spawn agents
6. **Monitor** — watch agents in the Agents tab, Fleet view, or see lifecycle events in chat
7. **Inspect** — click any agent to see structured event output (tool calls, text, commits, errors)

**Keyboard shortcuts:**
- `Cmd+K` — command palette (fuzzy search goals)
- `Cmd+N` — create new goal
- `Esc` — close dialogs

### Using the CLI

```sh
# Create a goal
conductor goal create "Add user authentication with JWT" --repo /path/to/project

# Decompose into tasks
conductor goal decompose <goal-id>

# Dispatch agents
conductor goal dispatch <goal-id>

# Monitor
conductor status
conductor logs <agent-id>

# Interact with a running agent
conductor nudge <agent-id> "Focus on the middleware first"
conductor kill <agent-id>

# Clean up stale worktrees and stuck agents
conductor cleanup
```

## Architecture

```
                    +---------------------+
                    |  Web UI (React)     |
                    |  Sidebar + Chat +   |
                    |  Fleet + Agent View |
                    +---------+-----------+
                              |
                    +---------+-----------+
                    |  Axum HTTP + SSE    |
                    |  REST API           |
                    +---------+-----------+
                              |
              +---------------+---------------+
              |               |               |
     +--------+----+  +------+-------+  +-----+------+
     | Agent       |  | Goal/Task    |  | SQLite DB  |
     | Manager     |  | Decomposer   |  | (goals,    |
     | (worktrees) |  | + Chat       |  |  tasks,    |
     +--------+----+  +--------------+  |  agents,   |
              |                         |  messages)  |
     +--------+----+------+------+     +------------+
     |             |      |      |
  +--+---+    +---+--+ +--+--+ +--+--+
  |agent |    |agent | |agent| |agent|
  |wt/1  |    |wt/2  | |wt/3 | |wt/4 |
  +------+    +------+ +-----+ +-----+
  (worktrees in /tmp/conductor/)
```

### Key Modules

| Module | Purpose |
|--------|---------|
| `src/agent/session.rs` | Spawns and monitors Claude Code processes |
| `src/agent/worktree.rs` | Creates isolated git worktrees per agent |
| `src/agent/event_parser.rs` | Parses Claude Code's stream-json NDJSON output |
| `src/goal/decompose.rs` | Uses Claude to break goals into dependency-ordered tasks |
| `src/goal/chat.rs` | Conversational goal chat with streamed responses |
| `src/server/routes.rs` | REST API + embedded frontend serving |
| `src/server/sse.rs` | Real-time event streaming (agent events, chat chunks) |
| `src/db/schema.rs` | SQLite migrations (goals, tasks, agents, events, projects, messages) |
| `src/db/queries.rs` | All database operations |
| `src/hooks/` | Claude Code hooks for agent lifecycle callbacks |

### Frontend Components

| Directory | Components |
|-----------|------------|
| `components/layout/` | SidebarLayout, Sidebar, ProjectSidebar |
| `components/chat/` | ChatView, ChatMessage, ChatInput, TaskProposal |
| `components/agent/` | EventRenderer, ToolCallEvent, TextEvent, CommitEvent, ErrorEvent |
| `components/ui/` | Button, Input, Textarea, Badge, Card, Dialog, Tabs, Select, ScrollArea, Skeleton, Tooltip |
| `components/` | FleetView, GoalSpaceView, AgentDetail, CostDashboard, CommandPalette, QuickTask, ProjectSettings |

### Agent Lifecycle

1. Task dispatched → git worktree created from repo
2. `claude -p` process spawned with task prompt + settings
3. NDJSON event stream parsed in real-time (tool calls, text, costs)
4. Events broadcast via SSE to all connected clients
5. On completion: branch merged, dependent tasks unblocked, auto-dispatch continues
6. Watchdog: stall detection (10min), hard timeout (20min), budget enforcement

### Per-Goal Settings

Each goal (and each task) can configure its agents:

| Setting | Default | Description |
|---------|---------|-------------|
| `model` | `sonnet` | Claude model (`sonnet`, `opus`, `haiku`, or full model ID) |
| `max_budget_usd` | `5.00` | Max spend per agent |
| `max_turns` | `50` | Max conversation turns |
| `allowed_tools` | Bash, Read, Edit, Write, Grep, Glob | Restrict tool access |
| `permission_mode` | default | Claude Code permission mode |
| `system_prompt` | — | Custom instructions appended to each agent |

Task-level settings override goal-level settings. Project-level settings provide defaults for all goals in a project.

## API Reference

All endpoints are under `/api/`. The server also serves the embedded React UI at the root path.

### Projects
```
GET    /api/projects                  List all projects
POST   /api/projects                  Create project
GET    /api/projects/:id              Get project
PUT    /api/projects/:id              Update project (name, settings)
DELETE /api/projects/:id              Delete project
GET    /api/projects/:id/goals        List goals for project
```

### Goals
```
POST   /api/goals                     Create goal
GET    /api/goals                     List all goals
GET    /api/goals/:id                 Get goal details
PUT    /api/goals/:id                 Update goal (name, description, status, settings)
DELETE /api/goals/:id                 Archive goal
POST   /api/goals/:id/decompose       Decompose into tasks (async, returns operation_id)
POST   /api/goals/:id/dispatch        Dispatch agents for unblocked tasks
POST   /api/goals/:id/retry-failed    Retry all failed tasks
```

### Chat
```
POST   /api/goals/:id/chat            Send message, streams AI response via SSE
GET    /api/goals/:id/messages         Get conversation history
```

### Tasks
```
GET    /api/goals/:id/tasks            List tasks for goal
POST   /api/goals/:id/tasks            Create task
PUT    /api/tasks/:id                  Update task
POST   /api/tasks/:id/retry            Retry failed task
POST   /api/tasks/:id/dispatch         Dispatch agent for single task
```

### Agents
```
GET    /api/agents                     List all agent runs
GET    /api/agents/:id                 Get agent details
POST   /api/agents/:id/nudge           Send message to running/completed agent
POST   /api/agents/:id/kill            Terminate agent
GET    /api/agents/:id/events          Get agent event history
```

### Streaming (SSE)
```
GET    /api/events                     Global event stream (agent_event, operation_update, chat_chunk)
GET    /api/agents/:id/stream          Per-agent event stream
```

### Stats
```
GET    /api/stats                      Fleet statistics (active agents, costs, task counts)
```

## Development

```sh
# Run frontend dev server (hot reload) + backend concurrently
make dev

# Run all checks
make test     # cargo test + vitest
make lint     # clippy + eslint
make check    # formatting (cargo fmt + prettier)

# Format code
make fmt

# Install git pre-commit hook (runs lint + format checks)
make setup-hooks

# Clean build artifacts
make clean
```

### Project Structure

```
conductor/
├── src/                            # Rust backend
│   ├── agent/                      # Claude Code process lifecycle
│   │   ├── session.rs              # Agent spawning, monitoring, SSE broadcast
│   │   ├── worktree.rs             # Git worktree management
│   │   └── event_parser.rs         # NDJSON stream parser
│   ├── server/                     # HTTP API, SSE, embedded UI
│   │   ├── routes.rs               # All REST endpoints
│   │   └── sse.rs                  # SSE event streaming
│   ├── goal/                       # Goal management
│   │   ├── decompose.rs            # AI task decomposition
│   │   ├── chat.rs                 # Conversational goal chat
│   │   ├── space.rs                # Goal space operations
│   │   └── task.rs                 # Task state machine
│   ├── db/                         # SQLite persistence
│   │   ├── schema.rs               # Migrations
│   │   └── queries.rs              # CRUD operations
│   ├── hooks/                      # Claude Code hooks
│   └── cli.rs                      # CLI command definitions
├── frontend/                       # React + Vite + TypeScript + Tailwind v4
│   └── src/
│       ├── components/
│       │   ├── layout/             # Sidebar, ProjectSidebar, SidebarLayout
│       │   ├── chat/               # ChatView, ChatMessage, ChatInput, TaskProposal
│       │   ├── agent/              # EventRenderer, ToolCallEvent, TextEvent, etc.
│       │   ├── ui/                 # shadcn/ui components (Button, Card, Dialog, etc.)
│       │   └── settings/           # ProjectSettings
│       ├── hooks/                  # SSE streaming, keyboard shortcuts
│       ├── lib/                    # Utilities (cn)
│       └── api/                    # API client
├── tests/                          # Integration tests
├── .github/workflows/              # CI and release pipelines
├── deny.toml                       # cargo-deny license/advisory config
└── Makefile                        # Build commands
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

## Release

Pushing a version tag triggers cross-platform binary builds:

```sh
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

# With launchd/systemd
conductor server --port 3001  # runs on port 3001 by default
```

## License

MIT
