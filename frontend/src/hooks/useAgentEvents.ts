import { useEffect, useRef, useState, useCallback } from 'react';
import type { AgentEvent, AgentRun, OperationUpdate } from '../types';

const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '/api';

interface UseAgentEventsOptions {
  agentId?: string;
}

interface AgentEventsState {
  events: AgentEvent[];
  agents: Map<string, AgentRun>;
  operations: Map<string, OperationUpdate>;
  operationLogs: Map<string, string[]>;
  connected: boolean;
}

export function useAgentEvents(options: UseAgentEventsOptions = {}) {
  const { agentId } = options;
  const [state, setState] = useState<AgentEventsState>({
    events: [],
    agents: new Map(),
    operations: new Map(),
    operationLogs: new Map(),
    connected: false,
  });
  const esRef = useRef<EventSource | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const connect = useCallback(() => {
    const url = agentId
      ? `${BASE_URL}/agents/${agentId}/stream`
      : `${BASE_URL}/events`;

    const es = new EventSource(url);
    esRef.current = es;

    es.onopen = () => {
      setState((s) => ({ ...s, connected: true }));
    };

    es.addEventListener('agent_update', (e) => {
      try {
        const agent: AgentRun = JSON.parse(e.data);
        setState((s) => {
          const agents = new Map(s.agents);
          agents.set(agent.id, agent);
          return { ...s, agents };
        });
      } catch { /* ignore malformed */ }
    });

    es.addEventListener('agent_event', (e) => {
      try {
        const event: AgentEvent = JSON.parse(e.data);
        setState((s) => ({
          ...s,
          events: [...s.events, event],
        }));
      } catch { /* ignore malformed */ }
    });

    es.addEventListener('operation_update', (e: MessageEvent) => {
      try {
        const op: OperationUpdate = JSON.parse(e.data);
        setState((s) => {
          const operations = new Map(s.operations);
          operations.set(op.operation_id, op);
          const operationLogs = new Map(s.operationLogs);
          if (op.status === 'running' && op.message) {
            const existing = operationLogs.get(op.operation_id) ?? [];
            operationLogs.set(op.operation_id, [...existing, op.message]);
          }
          return { ...s, operations, operationLogs };
        });
        // Auto-clean completed/failed operations after 30s
        if (op.status === 'completed' || op.status === 'failed') {
          setTimeout(() => {
            setState((s) => {
              const operations = new Map(s.operations);
              operations.delete(op.operation_id);
              const operationLogs = new Map(s.operationLogs);
              operationLogs.delete(op.operation_id);
              return { ...s, operations, operationLogs };
            });
          }, 30000);
        }
      } catch { /* ignore malformed */ }
    });

    es.addEventListener('message', (e) => {
      try {
        const data = JSON.parse(e.data);
        if (data.type === 'agent_update') {
          const agent: AgentRun = data.payload;
          setState((s) => {
            const agents = new Map(s.agents);
            agents.set(agent.id, agent);
            return { ...s, agents };
          });
        } else if (data.type === 'agent_event') {
          const event: AgentEvent = data.payload;
          setState((s) => ({
            ...s,
            events: [...s.events, event],
          }));
        }
      } catch { /* ignore */ }
    });

    es.onerror = () => {
      setState((s) => ({ ...s, connected: false }));
      es.close();
      reconnectTimer.current = setTimeout(connect, 3000);
    };
  }, [agentId]);

  useEffect(() => {
    connect();
    return () => {
      esRef.current?.close();
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
    };
  }, [connect]);

  return state;
}
