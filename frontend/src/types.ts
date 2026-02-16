export interface GoalSettings {
  model?: string;
  max_budget_usd?: number;
  max_turns?: number;
  allowed_tools?: string[];
  permission_mode?: string;
  system_prompt?: string;
}

export interface GoalSpace {
  id: string;
  name: string;
  description: string;
  status: "active" | "paused" | "completed" | "archived";
  repo_path: string;
  created_at: string;
  updated_at: string;
  settings: GoalSettings;
}

export interface Task {
  id: string;
  goal_space_id: string;
  title: string;
  description: string;
  status: "pending" | "assigned" | "running" | "done" | "failed" | "blocked";
  priority: number;
  depends_on: string[];
  created_at: string;
  updated_at: string;
}

export interface AgentRun {
  id: string;
  task_id: string;
  goal_space_id: string;
  claude_session_id: string | null;
  worktree_path: string | null;
  branch: string | null;
  status: "spawning" | "running" | "stalled" | "done" | "failed" | "killed";
  model: string;
  cost_usd: number;
  input_tokens: number;
  output_tokens: number;
  max_budget_usd: number | null;
  started_at: string;
  last_activity_at: string | null;
  finished_at: string | null;
}

export interface AgentEvent {
  id: number;
  agent_run_id: string;
  event_type:
    | "tool_call"
    | "tool_result"
    | "text_output"
    | "error"
    | "cost_update"
    | "stall"
    | "commit";
  tool_name: string | null;
  summary: string;
  raw_json: string | null;
  cost_delta_usd: number | null;
  created_at: string;
}

export interface Stats {
  active_agents: number;
  total_cost_usd: number;
  tasks_completed: number;
  tasks_total: number;
  goals_active: number;
}

export interface OperationUpdate {
  kind: "OperationUpdate";
  operation_id: string;
  goal_space_id: string;
  operation_type: "decompose" | "dispatch";
  status: "running" | "completed" | "failed";
  message: string;
  result: unknown | null;
}

export interface OperationStarted {
  operation_id: string;
  status: "running";
  message?: string;
}
