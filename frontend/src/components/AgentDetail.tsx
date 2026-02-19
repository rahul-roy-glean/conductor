import { useEffect, useRef, useState } from "react";
import { useParams } from "react-router-dom";
import { getAgent, getAgentEvents } from "@/api/client";
import { useAgentEvents } from "@/hooks/useAgentEvents";
import type { AgentRun, AgentEvent } from "@/types";
import NudgeDialog from "@/components/NudgeDialog";
import { Zap } from "lucide-react";
import { useToast } from "@/components/ToastProvider";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import EventRenderer from "@/components/agent/EventRenderer";

const statusBadge: Record<string, string> = {
  spawning: "bg-blue-900 text-blue-300",
  running: "bg-green-900 text-green-300",
  stalled: "bg-yellow-900 text-yellow-300",
  done: "bg-muted text-foreground",
  failed: "bg-red-900 text-red-300",
  killed: "bg-red-900 text-red-300",
};

export default function AgentDetail() {
  const { id } = useParams<{ id: string }>();
  const [agent, setAgent] = useState<AgentRun | null>(null);
  const [events, setEvents] = useState<AgentEvent[]>([]);
  const [showNudge, setShowNudge] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const { events: liveEvents } = useAgentEvents({ agentId: id });
  const { addToast } = useToast();

  useEffect(() => {
    if (!id) return;
    getAgent(id)
      .then(setAgent)
      .catch(() => addToast("error", "Failed to load agent"));
    getAgentEvents(id)
      .then(setEvents)
      .catch(() => addToast("error", "Failed to load events"));
  }, [id]);

  const allEvents = [
    ...events,
    ...liveEvents.filter(
      (le) => le.agent_run_id === id && !events.find((e) => e.id === le.id),
    ),
  ];

  useEffect(() => {
    scrollRef.current?.scrollTo(0, scrollRef.current.scrollHeight);
  }, [allEvents.length]);

  if (!agent) {
    return <p className="text-muted-foreground p-8">Loading agent...</p>;
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-bold text-foreground font-mono">
              {agent.branch ?? agent.id.slice(0, 8)}
            </h1>
            <Badge
              variant="outline"
              className={statusBadge[agent.status] ?? ""}
            >
              {agent.status}
            </Badge>
          </div>
          <p className="text-xs text-muted-foreground font-mono mt-1">
            {agent.id}
          </p>
          <p className="text-xs text-muted-foreground mt-0.5">
            Model: {agent.model}
            {agent.worktree_path && <> &middot; {agent.worktree_path}</>}
          </p>
        </div>
        {(agent.status === "running" || agent.status === "stalled") && (
          <Button onClick={() => setShowNudge(true)}>
            <Zap size={14} /> Nudge
          </Button>
        )}
      </div>

      {/* Cost breakdown */}
      <div className="grid grid-cols-3 gap-4">
        <Card className="p-4">
          <span className="text-xs text-muted-foreground">Input Tokens</span>
          <p className="text-lg font-mono text-foreground">
            {agent.input_tokens.toLocaleString()}
          </p>
        </Card>
        <Card className="p-4">
          <span className="text-xs text-muted-foreground">Output Tokens</span>
          <p className="text-lg font-mono text-foreground">
            {agent.output_tokens.toLocaleString()}
          </p>
        </Card>
        <Card className="p-4">
          <span className="text-xs text-muted-foreground">Total Cost</span>
          <p className="text-lg font-mono text-green-400">
            ${agent.cost_usd.toFixed(4)}
          </p>
        </Card>
      </div>

      {/* Rich event timeline */}
      <div>
        <h2 className="text-lg font-semibold text-foreground mb-3">Activity</h2>
        <ScrollArea
          className={cn("max-h-[600px]", allEvents.length > 0 && "h-[600px]")}
        >
          <div ref={scrollRef} className="space-y-0.5 px-1">
            {allEvents.length === 0 && (
              <p className="text-muted-foreground text-sm py-4">
                No events yet
              </p>
            )}
            {allEvents.map((event) => (
              <div key={event.id} className="flex gap-3">
                <span className="text-[10px] text-muted-foreground font-mono min-w-[60px] pt-2.5 shrink-0">
                  {new Date(event.created_at).toLocaleTimeString()}
                </span>
                <div className="flex-1 min-w-0">
                  <EventRenderer event={event} />
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>
      </div>

      {showNudge && (
        <NudgeDialog agentId={agent.id} onClose={() => setShowNudge(false)} />
      )}
    </div>
  );
}
