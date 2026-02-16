import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import AgentCard from './AgentCard';
import type { AgentRun } from '../types';

// Mock the API client to avoid real fetch calls
vi.mock('../api/client', () => ({
  killAgent: vi.fn(() => Promise.resolve()),
  nudgeAgent: vi.fn(() => Promise.resolve()),
}));

function makeAgent(overrides: Partial<AgentRun> = {}): AgentRun {
  return {
    id: 'agent-abc-12345678',
    task_id: 'task-1',
    goal_space_id: 'goal-1',
    claude_session_id: null,
    worktree_path: null,
    branch: null,
    status: 'running',
    model: 'claude-3',
    cost_usd: 1.23,
    input_tokens: 100,
    output_tokens: 50,
    max_budget_usd: null,
    started_at: new Date().toISOString(),
    last_activity_at: null,
    finished_at: null,
    ...overrides,
  };
}

function renderCard(agent: AgentRun, props: { taskTitle?: string; lastActivity?: string; onRefresh?: () => void } = {}) {
  return render(
    <MemoryRouter>
      <AgentCard agent={agent} {...props} />
    </MemoryRouter>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('AgentCard', () => {
  it('renders agent branch when available', () => {
    const agent = makeAgent({ branch: 'feature/my-branch' });
    renderCard(agent);
    expect(screen.getByText('feature/my-branch')).toBeInTheDocument();
  });

  it('renders agent id prefix when no branch', () => {
    const agent = makeAgent({ branch: null, id: 'abcdef1234567890' });
    renderCard(agent);
    expect(screen.getByText('abcdef12')).toBeInTheDocument();
  });

  it('renders task title when provided', () => {
    const agent = makeAgent();
    renderCard(agent, { taskTitle: 'Implement feature X' });
    expect(screen.getByText('Implement feature X')).toBeInTheDocument();
  });

  it('shows correct status color dot for running (green)', () => {
    const agent = makeAgent({ status: 'running' });
    const { container } = renderCard(agent);
    const dot = container.querySelector('.bg-green-500');
    expect(dot).toBeInTheDocument();
  });

  it('shows correct status color dot for stalled (yellow)', () => {
    const agent = makeAgent({ status: 'stalled' });
    const { container } = renderCard(agent);
    const dot = container.querySelector('.bg-yellow-500');
    expect(dot).toBeInTheDocument();
  });

  it('shows correct status color dot for done (gray)', () => {
    const agent = makeAgent({ status: 'done' });
    const { container } = renderCard(agent);
    const dot = container.querySelector('.bg-gray-500');
    expect(dot).toBeInTheDocument();
  });

  it('shows correct status color dot for failed (red)', () => {
    const agent = makeAgent({ status: 'failed' });
    const { container } = renderCard(agent);
    const dot = container.querySelector('.bg-red-500');
    expect(dot).toBeInTheDocument();
  });

  it('shows correct status color dot for spawning (blue)', () => {
    const agent = makeAgent({ status: 'spawning' });
    const { container } = renderCard(agent);
    const dot = container.querySelector('.bg-blue-500');
    expect(dot).toBeInTheDocument();
  });

  it('shows correct status color dot for killed (red-700)', () => {
    const agent = makeAgent({ status: 'killed' });
    const { container } = renderCard(agent);
    const dot = container.querySelector('.bg-red-700');
    expect(dot).toBeInTheDocument();
  });

  it('shows cost formatted as dollar amount', () => {
    const agent = makeAgent({ cost_usd: 4.56 });
    renderCard(agent);
    expect(screen.getByText('$4.56')).toBeInTheDocument();
  });

  it('shows cost formatted with two decimal places', () => {
    const agent = makeAgent({ cost_usd: 0.1 });
    renderCard(agent);
    expect(screen.getByText('$0.10')).toBeInTheDocument();
  });

  it('shows status text', () => {
    const agent = makeAgent({ status: 'running' });
    renderCard(agent);
    expect(screen.getByText('running')).toBeInTheDocument();
  });

  it('shows nudge and kill action buttons', () => {
    const agent = makeAgent();
    renderCard(agent);
    expect(screen.getByText('Nudge')).toBeInTheDocument();
    expect(screen.getByText('Kill')).toBeInTheDocument();
  });

  it('shows last activity when provided', () => {
    const agent = makeAgent();
    renderCard(agent, { lastActivity: 'Wrote a test file' });
    expect(screen.getByText('Wrote a test file')).toBeInTheDocument();
  });

  it('card is clickable (has cursor-pointer class)', () => {
    const agent = makeAgent();
    const { container } = renderCard(agent);
    const card = container.querySelector('.cursor-pointer');
    expect(card).toBeInTheDocument();
  });
});
