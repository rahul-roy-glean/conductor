import type { GoalSpace, GoalSettings, Task, AgentRun, AgentEvent, Stats, OperationStarted } from '../types';

const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '/api';

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...init,
  });
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new Error(`API ${res.status}: ${body}`);
  }
  return res.json() as Promise<T>;
}

// --- Goals ---

export function listGoals(): Promise<GoalSpace[]> {
  return request('/goals');
}

export function getGoal(id: string): Promise<GoalSpace> {
  return request(`/goals/${id}`);
}

export function createGoal(data: { name: string; description: string; repo_path: string }): Promise<GoalSpace> {
  return request('/goals', { method: 'POST', body: JSON.stringify(data) });
}

export function updateGoal(id: string, data: Partial<Pick<GoalSpace, 'name' | 'description' | 'status'>> & { settings?: GoalSettings }): Promise<GoalSpace> {
  return request(`/goals/${id}`, { method: 'PUT', body: JSON.stringify(data) });
}

export function deleteGoal(id: string): Promise<void> {
  return request(`/goals/${id}`, { method: 'DELETE' });
}

// --- Tasks ---

export function listTasks(goalId: string): Promise<Task[]> {
  return request(`/goals/${goalId}/tasks`);
}

export function getTask(goalId: string, taskId: string): Promise<Task> {
  return request(`/goals/${goalId}/tasks/${taskId}`);
}

export function createTask(goalId: string, data: { title: string; description: string; priority?: number; depends_on?: string[] }): Promise<Task> {
  return request(`/goals/${goalId}/tasks`, { method: 'POST', body: JSON.stringify(data) });
}

export function updateTask(_goalId: string, taskId: string, data: Partial<Pick<Task, 'title' | 'description' | 'status' | 'priority'>>): Promise<Task> {
  return request(`/tasks/${taskId}`, { method: 'PUT', body: JSON.stringify(data) });
}

export function deleteTask(goalId: string, taskId: string): Promise<void> {
  return request(`/goals/${goalId}/tasks/${taskId}`, { method: 'DELETE' });
}

export function retryTask(taskId: string): Promise<void> {
  return request(`/tasks/${taskId}/retry`, { method: 'POST' });
}

export function retryAllFailed(goalId: string): Promise<{ ok: boolean; retried: number }> {
  return request(`/goals/${goalId}/retry-failed`, { method: 'POST' });
}

// --- Agents ---

export function listAgents(): Promise<AgentRun[]> {
  return request('/agents');
}

export function getAgent(id: string): Promise<AgentRun> {
  return request(`/agents/${id}`);
}

export function getAgentEvents(id: string): Promise<AgentEvent[]> {
  return request(`/agents/${id}/events`);
}

export function nudgeAgent(id: string, message: string): Promise<void> {
  return request(`/agents/${id}/nudge`, { method: 'POST', body: JSON.stringify({ message }) });
}

export function killAgent(id: string): Promise<void> {
  return request(`/agents/${id}/kill`, { method: 'POST' });
}

// --- Stats ---

export function getStats(): Promise<Stats> {
  return request('/stats');
}

// --- Goal actions ---

export function decomposeGoal(goalId: string): Promise<OperationStarted> {
  return request(`/goals/${goalId}/decompose`, { method: 'POST' });
}

export function dispatchGoal(goalId: string): Promise<OperationStarted> {
  return request(`/goals/${goalId}/dispatch`, { method: 'POST' });
}

export function dispatchTask(taskId: string): Promise<OperationStarted> {
  return request(`/tasks/${taskId}/dispatch`, { method: 'POST' });
}
