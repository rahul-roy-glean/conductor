# API Reference

All endpoints are under `/api/`. The server also serves the embedded React UI at the root path.

## Projects

```
GET    /api/projects                  List all projects
POST   /api/projects                  Create project
GET    /api/projects/:id              Get project
PUT    /api/projects/:id              Update project (name, settings)
DELETE /api/projects/:id              Delete project
GET    /api/projects/:id/goals        List goals for project
```

## Goals

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

## Chat

```
POST   /api/goals/:id/chat            Send message, streams AI response via SSE
GET    /api/goals/:id/messages         Get conversation history
```

## Tasks

```
GET    /api/goals/:id/tasks            List tasks for goal
POST   /api/goals/:id/tasks            Create task
PUT    /api/tasks/:id                  Update task
POST   /api/tasks/:id/retry            Retry failed task
POST   /api/tasks/:id/dispatch         Dispatch agent for single task
```

## Agents

```
GET    /api/agents                     List all agent runs
GET    /api/agents/:id                 Get agent details
POST   /api/agents/:id/nudge           Send message to running/completed agent
POST   /api/agents/:id/kill            Terminate agent
GET    /api/agents/:id/events          Get agent event history
```

## Streaming (SSE)

```
GET    /api/events                     Global event stream (agent_event, operation_update, chat_chunk)
GET    /api/agents/:id/stream          Per-agent event stream
```

## Stats

```
GET    /api/stats                      Fleet statistics (active agents, costs, task counts)
```

## Per-Goal Settings

Each goal (and each task) can configure its agents. Set via `PUT /api/goals/:id` with a `settings` object:

| Setting | Default | Description |
|---------|---------|-------------|
| `model` | `sonnet` | Claude model (`sonnet`, `opus`, `haiku`, or full model ID) |
| `max_budget_usd` | `5.00` | Max spend per agent |
| `max_turns` | `50` | Max conversation turns |
| `allowed_tools` | Bash, Read, Edit, Write, Grep, Glob | Restrict tool access |
| `permission_mode` | default | Claude Code permission mode |
| `system_prompt` | — | Custom instructions appended to each agent |

Task-level settings override goal-level settings. Project-level settings provide defaults for all goals in a project.
