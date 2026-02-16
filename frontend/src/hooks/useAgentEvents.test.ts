import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useAgentEvents } from './useAgentEvents';

type EventHandler = ((event: MessageEvent) => void) | (() => void) | null;

class MockEventSource {
  url: string;
  onopen: EventHandler = null;
  onerror: EventHandler = null;
  listeners: Record<string, EventHandler[]> = {};
  closeCalled = false;

  constructor(url: string) {
    this.url = url;
    MockEventSource.instances.push(this);
  }

  addEventListener(type: string, handler: EventHandler) {
    if (!this.listeners[type]) this.listeners[type] = [];
    this.listeners[type].push(handler);
  }

  removeEventListener() {
    // no-op for tests
  }

  close() {
    this.closeCalled = true;
  }

  // Helper to simulate events
  simulateOpen() {
    if (this.onopen) (this.onopen as () => void)();
  }

  simulateError() {
    if (this.onerror) (this.onerror as () => void)();
  }

  simulateEvent(type: string, data: string) {
    const event = { data } as MessageEvent;
    const handlers = this.listeners[type] ?? [];
    for (const handler of handlers) {
      if (handler) (handler as (e: MessageEvent) => void)(event);
    }
  }

  static instances: MockEventSource[] = [];
  static reset() {
    MockEventSource.instances = [];
  }
}

beforeEach(() => {
  MockEventSource.reset();
  vi.useFakeTimers();
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (globalThis as any).EventSource = MockEventSource;
});

afterEach(() => {
  vi.useRealTimers();
});

describe('useAgentEvents', () => {
  it('creates EventSource with correct URL (no agentId)', () => {
    renderHook(() => useAgentEvents());
    expect(MockEventSource.instances).toHaveLength(1);
    expect(MockEventSource.instances[0].url).toBe('/api/events');
  });

  it('creates EventSource with agent-specific URL', () => {
    renderHook(() => useAgentEvents({ agentId: 'abc-123' }));
    expect(MockEventSource.instances).toHaveLength(1);
    expect(MockEventSource.instances[0].url).toBe('/api/agents/abc-123/stream');
  });

  it('connected state starts false, becomes true on open', () => {
    const { result } = renderHook(() => useAgentEvents());

    expect(result.current.connected).toBe(false);

    act(() => {
      MockEventSource.instances[0].simulateOpen();
    });

    expect(result.current.connected).toBe(true);
  });

  it('agent_update events are parsed and stored in agents map', () => {
    const { result } = renderHook(() => useAgentEvents());

    const agentData = {
      id: 'agent-1',
      task_id: 'task-1',
      goal_space_id: 'goal-1',
      claude_session_id: null,
      worktree_path: null,
      branch: null,
      status: 'running',
      model: 'claude-3',
      cost_usd: 0.5,
      input_tokens: 100,
      output_tokens: 50,
      max_budget_usd: null,
      started_at: '2025-01-01T00:00:00Z',
      last_activity_at: null,
      finished_at: null,
    };

    act(() => {
      MockEventSource.instances[0].simulateEvent('agent_update', JSON.stringify(agentData));
    });

    expect(result.current.agents.size).toBe(1);
    expect(result.current.agents.get('agent-1')).toEqual(agentData);
  });

  it('agent_event events are parsed and stored in events array', () => {
    const { result } = renderHook(() => useAgentEvents());

    const eventData = {
      id: 1,
      agent_run_id: 'agent-1',
      event_type: 'text_output',
      tool_name: null,
      summary: 'Some output',
      raw_json: null,
      cost_delta_usd: null,
      created_at: '2025-01-01T00:00:00Z',
    };

    act(() => {
      MockEventSource.instances[0].simulateEvent('agent_event', JSON.stringify(eventData));
    });

    expect(result.current.events).toHaveLength(1);
    expect(result.current.events[0]).toEqual(eventData);
  });

  it('on error, connected becomes false', () => {
    const { result } = renderHook(() => useAgentEvents());

    act(() => {
      MockEventSource.instances[0].simulateOpen();
    });
    expect(result.current.connected).toBe(true);

    act(() => {
      MockEventSource.instances[0].simulateError();
    });
    expect(result.current.connected).toBe(false);
  });

  it('cleanup closes EventSource on unmount', () => {
    const { unmount } = renderHook(() => useAgentEvents());

    const es = MockEventSource.instances[0];
    expect(es.closeCalled).toBe(false);

    unmount();
    expect(es.closeCalled).toBe(true);
  });
});
