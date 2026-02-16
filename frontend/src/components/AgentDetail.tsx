import { useEffect, useRef, useState } from 'react';
import { useParams } from 'react-router-dom';
import { getAgent, getAgentEvents } from '../api/client';
import { useAgentEvents } from '../hooks/useAgentEvents';
import type { AgentRun, AgentEvent } from '../types';
import NudgeDialog from './NudgeDialog';
import {
  Terminal, AlertTriangle, GitCommit, DollarSign, Wrench, FileText, Zap,
} from 'lucide-react';
import { useToast } from './ToastProvider';

const eventIcon: Record<AgentEvent['event_type'], typeof Terminal> = {
  tool_call: Wrench,
  tool_result: FileText,
  text_output: Terminal,
  error: AlertTriangle,
  cost_update: DollarSign,
  stall: AlertTriangle,
  commit: GitCommit,
};

export default function AgentDetail() {
  const { id } = useParams<{ id: string }>();
  const [agent, setAgent] = useState<AgentRun | null>(null);
  const [events, setEvents] = useState<AgentEvent[]>([]);
  const [showNudge, setShowNudge] = useState(false);
  const outputRef = useRef<HTMLDivElement>(null);
  const { events: liveEvents } = useAgentEvents({ agentId: id });
  const { addToast } = useToast();

  useEffect(() => {
    if (!id) return;
    getAgent(id).then(setAgent).catch(() => addToast('error', 'Failed to load agent'));
    getAgentEvents(id).then(setEvents).catch(() => addToast('error', 'Failed to load events'));
  }, [id]);

  const allEvents = [...events, ...liveEvents.filter(
    (le) => le.agent_run_id === id && !events.find((e) => e.id === le.id),
  )];

  useEffect(() => {
    outputRef.current?.scrollTo(0, outputRef.current.scrollHeight);
  }, [allEvents.length]);

  if (!agent) {
    return <p className="text-gray-400 p-8">Loading agent...</p>;
  }

  const textEvents = allEvents.filter((e) => e.event_type === 'text_output');

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-100">
            {agent.branch ?? agent.id.slice(0, 8)}
          </h1>
          <p className="text-sm text-gray-400">{agent.id}</p>
        </div>
        <button
          onClick={() => setShowNudge(true)}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded text-white text-sm transition-colors"
        >
          <Zap size={14} /> Nudge
        </button>
      </div>

      {/* Cost breakdown */}
      <div className="grid grid-cols-3 gap-4">
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <span className="text-xs text-gray-400">Input Tokens</span>
          <p className="text-lg font-mono text-gray-100">{agent.input_tokens.toLocaleString()}</p>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <span className="text-xs text-gray-400">Output Tokens</span>
          <p className="text-lg font-mono text-gray-100">{agent.output_tokens.toLocaleString()}</p>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <span className="text-xs text-gray-400">Total Cost</span>
          <p className="text-lg font-mono text-green-400">${agent.cost_usd.toFixed(4)}</p>
        </div>
      </div>

      {/* Event timeline */}
      <div>
        <h2 className="text-lg font-semibold text-gray-100 mb-3">Event Timeline</h2>
        <div className="space-y-1 max-h-96 overflow-y-auto bg-gray-800 rounded-lg border border-gray-700 p-4">
          {allEvents.length === 0 && (
            <p className="text-gray-500 text-sm">No events yet</p>
          )}
          {allEvents.map((event) => {
            const Icon = eventIcon[event.event_type] ?? Terminal;
            return (
              <div key={event.id} className="flex items-start gap-3 py-1.5">
                <span className="text-xs text-gray-500 font-mono min-w-[70px] pt-0.5">
                  {new Date(event.created_at).toLocaleTimeString()}
                </span>
                <Icon size={14} className="text-gray-400 mt-0.5 shrink-0" />
                <span className="text-sm text-gray-300">{event.summary}</span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Live output */}
      <div>
        <h2 className="text-lg font-semibold text-gray-100 mb-3">Live Output</h2>
        <div
          ref={outputRef}
          className="bg-gray-950 rounded-lg border border-gray-700 p-4 font-mono text-xs text-green-400 max-h-80 overflow-y-auto whitespace-pre-wrap"
        >
          {textEvents.length === 0 && (
            <span className="text-gray-600">Waiting for output...</span>
          )}
          {textEvents.map((e) => (
            <div key={e.id}>{e.summary}</div>
          ))}
        </div>
      </div>

      {showNudge && (
        <NudgeDialog agentId={agent.id} onClose={() => setShowNudge(false)} />
      )}
    </div>
  );
}
