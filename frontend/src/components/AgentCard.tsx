import { useNavigate } from "react-router-dom";
import { Zap, Skull, Loader2 } from "lucide-react";
import type { AgentRun } from "@/types";
import { killAgent } from "@/api/client";
import { useState } from "react";
import NudgeDialog from "@/components/NudgeDialog";
import { useToast } from "@/components/ToastProvider";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const statusColor: Record<AgentRun["status"], string> = {
  spawning: "bg-blue-500",
  running: "bg-green-500 animate-pulse",
  stalled: "bg-yellow-500",
  done: "bg-gray-500",
  failed: "bg-red-500",
  killed: "bg-red-700",
};

function elapsed(started: string, finished: string | null): string {
  const start = new Date(started).getTime();
  const end = finished ? new Date(finished).getTime() : Date.now();
  const seconds = Math.floor((end - start) / 1000);
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return `${h}h ${m}m`;
}

interface Props {
  agent: AgentRun;
  taskTitle?: string;
  lastActivity?: string;
  onRefresh?: () => void;
}

export default function AgentCard({
  agent,
  taskTitle,
  lastActivity,
  onRefresh,
}: Props) {
  const navigate = useNavigate();
  const [showNudge, setShowNudge] = useState(false);
  const [killing, setKilling] = useState(false);
  const { addToast } = useToast();

  const handleKill = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setKilling(true);
    try {
      await killAgent(agent.id);
      onRefresh?.();
    } catch {
      addToast("error", "Failed to kill agent");
    } finally {
      setKilling(false);
    }
  };

  return (
    <>
      <Card
        onClick={() => navigate(`/agents/${agent.id}`)}
        className="p-4 cursor-pointer hover:bg-muted/50 hover:border-input transition-colors"
      >
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <span
              className={cn(
                "w-2.5 h-2.5 rounded-full",
                statusColor[agent.status],
              )}
            />
            <span className="text-sm font-medium text-foreground truncate">
              {agent.branch ?? agent.id.slice(0, 8)}
            </span>
          </div>
          <span className="text-xs text-muted-foreground">{agent.status}</span>
        </div>

        {taskTitle && (
          <p className="text-sm text-foreground truncate mb-2">{taskTitle}</p>
        )}

        <div className="flex items-center justify-between text-xs text-muted-foreground mb-3">
          <span>{elapsed(agent.started_at, agent.finished_at)}</span>
          <span className="font-mono">${agent.cost_usd.toFixed(2)}</span>
        </div>

        {lastActivity && (
          <p className="text-xs text-muted-foreground truncate mb-3">{lastActivity}</p>
        )}

        <div className="flex gap-2">
          <Button
            variant="ghost"
            size="sm"
            className="text-yellow-400"
            onClick={(e) => {
              e.stopPropagation();
              setShowNudge(true);
            }}
          >
            <Zap size={12} /> Nudge
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="text-red-400 hover:bg-red-900/30"
            onClick={handleKill}
            disabled={killing}
          >
            {killing ? (
              <Loader2 size={12} className="animate-spin" />
            ) : (
              <Skull size={12} />
            )}{" "}
            Kill
          </Button>
        </div>
      </Card>

      {showNudge && (
        <NudgeDialog agentId={agent.id} onClose={() => setShowNudge(false)} />
      )}
    </>
  );
}
