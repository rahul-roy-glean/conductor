# Architecture

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

## Backend Modules

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

## Frontend Components

| Directory | Components |
|-----------|------------|
| `components/layout/` | SidebarLayout, Sidebar, ProjectSidebar |
| `components/chat/` | ChatView, ChatMessage, ChatInput, TaskProposal |
| `components/agent/` | EventRenderer, ToolCallEvent, TextEvent, CommitEvent, ErrorEvent |
| `components/ui/` | Button, Input, Textarea, Badge, Card, Dialog, Tabs, Select, ScrollArea, Skeleton, Tooltip |
| `components/` | FleetView, GoalSpaceView, AgentDetail, CostDashboard, CommandPalette, QuickTask, ProjectSettings |

## Agent Lifecycle

1. Task dispatched → git worktree created from repo
2. `claude -p` process spawned with task prompt + settings
3. NDJSON event stream parsed in real-time (tool calls, text, costs)
4. Events broadcast via SSE to all connected clients
5. On completion: branch merged, dependent tasks unblocked, auto-dispatch continues
6. Watchdog: stall detection (10min), hard timeout (20min), budget enforcement

## Project Structure

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
