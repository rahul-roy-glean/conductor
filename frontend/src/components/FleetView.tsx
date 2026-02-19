import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { listAgents, listGoals, getStats, killAgent } from "@/api/client";
import type { AgentRun, GoalSpace, Stats } from "@/types";
import { useAgentEvents } from "@/hooks/useAgentEvents";
import {
  Activity,
  DollarSign,
  CheckCircle,
  Monitor,
  Zap,
  Skull,
  Loader2,
  FolderGit2,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import { useToast } from "@/components/ToastProvider";
import NudgeDialog from "@/components/NudgeDialog";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const statusColor: Record<AgentRun["status"], string> = {
  spawning: "text-blue-400",
  running: "text-green-400",
  stalled: "text-yellow-400",
  done: "text-muted-foreground",
  failed: "text-red-400",
  killed: "text-red-600",
};

const statusDot: Record<AgentRun["status"], string> = {
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

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diff = Math.floor((now - then) / 1000);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

export default function FleetView() {
  const [agents, setAgents] = useState<AgentRun[]>([]);
  const [goals, setGoals] = useState<GoalSpace[]>([]);
  const [stats, setStats] = useState<Stats | null>(null);
  const [nudgeId, setNudgeId] = useState<string | null>(null);
  const [killingId, setKillingId] = useState<string | null>(null);
  const [collapsedRepos, setCollapsedRepos] = useState<Set<string>>(new Set());
  const { agents: liveAgents, connected } = useAgentEvents();
  const { addToast } = useToast();
  const navigate = useNavigate();

  const loadData = () => {
    listAgents()
      .then(setAgents)
      .catch(() => addToast("error", "Failed to load agents"));
    listGoals()
      .then(setGoals)
      .catch(() => addToast("error", "Failed to load goals"));
    getStats()
      .then(setStats)
      .catch(() => addToast("error", "Failed to load stats"));
  };

  useEffect(() => {
    loadData();
  }, []);

  // Merge live agent updates into the list
  const mergedAgents = agents.map((a) => liveAgents.get(a.id) ?? a);
  for (const [id, agent] of liveAgents) {
    if (!agents.find((a) => a.id === id)) {
      mergedAgents.push(agent);
    }
  }

  // Build goal lookup for repo paths
  const goalMap = new Map<string, GoalSpace>();
  for (const g of goals) {
    goalMap.set(g.id, g);
  }

  // Group agents by repo path
  const grouped = new Map<string, AgentRun[]>();
  for (const agent of mergedAgents) {
    const goal = goalMap.get(agent.goal_space_id);
    const repo = goal?.repo_path ?? "Unknown Repository";
    if (!grouped.has(repo)) grouped.set(repo, []);
    grouped.get(repo)!.push(agent);
  }

  // Sort groups: active agents first, then by repo path
  const sortedGroups = [...grouped.entries()].sort(([, a], [, b]) => {
    const aActive = a.some(
      (ag) => ag.status === "running" || ag.status === "spawning",
    );
    const bActive = b.some(
      (ag) => ag.status === "running" || ag.status === "spawning",
    );
    if (aActive && !bActive) return -1;
    if (!aActive && bActive) return 1;
    return 0;
  });

  const handleKill = async (agentId: string) => {
    setKillingId(agentId);
    try {
      await killAgent(agentId);
      loadData();
    } catch {
      addToast("error", "Failed to kill agent");
    } finally {
      setKillingId(null);
    }
  };

  const toggleRepo = (repoPath: string) => {
    setCollapsedRepos((prev) => {
      const next = new Set(prev);
      if (next.has(repoPath)) {
        next.delete(repoPath);
      } else {
        next.add(repoPath);
      }
      return next;
    });
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-foreground">Fleet</h1>
        <div className="flex items-center gap-1.5">
          <span
            className={cn(
              "w-2 h-2 rounded-full",
              connected ? "bg-green-500" : "bg-red-500",
            )}
          />
          <span className="text-xs text-muted-foreground">
            {connected ? "Live" : "Disconnected"}
          </span>
        </div>
      </div>

      {/* Stats bar */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <Card className="p-4">
          <div className="flex items-center gap-2 text-muted-foreground mb-1">
            <Activity size={16} />
            <span className="text-xs">Active Agents</span>
          </div>
          <span className="text-2xl font-bold text-foreground">
            {stats?.active_agents ?? 0}
          </span>
        </Card>
        <Card className="p-4">
          <div className="flex items-center gap-2 text-muted-foreground mb-1">
            <DollarSign size={16} />
            <span className="text-xs">Total Cost</span>
          </div>
          <span className="text-2xl font-bold text-foreground font-mono">
            ${(stats?.total_cost_usd ?? 0).toFixed(2)}
          </span>
        </Card>
        <Card className="p-4">
          <div className="flex items-center gap-2 text-muted-foreground mb-1">
            <CheckCircle size={16} />
            <span className="text-xs">Tasks Done</span>
          </div>
          <span className="text-2xl font-bold text-foreground">
            {stats?.tasks_completed ?? 0}/{stats?.tasks_total ?? 0}
          </span>
        </Card>
      </div>

      {/* Agent list grouped by repo */}
      {sortedGroups.map(([repoPath, repoAgents]) => {
        const activeCount = repoAgents.filter(
          (a) => a.status === "running" || a.status === "spawning",
        ).length;
        const totalCost = repoAgents.reduce((sum, a) => sum + a.cost_usd, 0);
        const isCollapsed = collapsedRepos.has(repoPath);

        return (
          <div key={repoPath} className="mb-6">
            {/* Repo header - clickable to collapse/expand */}
            <button
              onClick={() => toggleRepo(repoPath)}
              className="flex items-center gap-2 mb-2 px-2 py-1.5 w-full rounded hover:bg-card transition-colors group"
            >
              {isCollapsed ? (
                <ChevronRight
                  size={14}
                  className="text-muted-foreground shrink-0"
                />
              ) : (
                <ChevronDown
                  size={14}
                  className="text-muted-foreground shrink-0"
                />
              )}
              <FolderGit2
                size={14}
                className="text-muted-foreground shrink-0"
              />
              <span className="text-sm font-mono text-foreground truncate">
                {repoPath}
              </span>
              <span className="text-xs text-muted-foreground shrink-0">
                {repoAgents.length} agent{repoAgents.length !== 1 ? "s" : ""}
                {activeCount > 0 && (
                  <span className="text-green-400 ml-1">
                    • {activeCount} active
                  </span>
                )}
              </span>
              <span className="text-xs text-muted-foreground shrink-0 ml-auto font-mono">
                ${totalCost.toFixed(2)}
              </span>
            </button>

            {/* Agent table - only show if not collapsed */}
            {!isCollapsed && (
              <Card className="overflow-hidden">
                {/* Header row */}
                <div className="grid grid-cols-[auto_1fr_100px_80px_100px_100px_auto] gap-3 items-center px-4 py-2 text-xs text-muted-foreground border-b border-border font-medium">
                  <span className="w-2.5" />
                  <span>Branch</span>
                  <span>Status</span>
                  <span className="text-right">Elapsed</span>
                  <span className="text-right">Cost</span>
                  <span className="text-right">Started</span>
                  <span />
                </div>

                {/* Agent rows */}
                {repoAgents.map((agent) => (
                  <div
                    key={agent.id}
                    onClick={() => navigate(`/agents/${agent.id}`)}
                    className="grid grid-cols-[auto_1fr_100px_80px_100px_100px_auto] gap-3 items-center px-4 py-2.5 border-b border-border/50 last:border-b-0 hover:bg-muted/50 cursor-pointer transition-colors group"
                  >
                    {/* Status dot */}
                    <span
                      className={cn(
                        "w-2.5 h-2.5 rounded-full shrink-0",
                        statusDot[agent.status],
                      )}
                    />

                    {/* Branch name */}
                    <span className="text-sm font-mono text-foreground truncate">
                      {agent.branch ?? agent.id.slice(0, 12)}
                    </span>

                    {/* Status */}
                    <span
                      className={cn(
                        "text-xs font-medium",
                        statusColor[agent.status],
                      )}
                    >
                      {agent.status}
                    </span>

                    {/* Elapsed */}
                    <span className="text-xs text-muted-foreground text-right font-mono">
                      {elapsed(agent.started_at, agent.finished_at)}
                    </span>

                    {/* Cost */}
                    <span className="text-xs text-muted-foreground text-right font-mono">
                      ${agent.cost_usd.toFixed(2)}
                    </span>

                    {/* Started timestamp */}
                    <span
                      className="text-xs text-muted-foreground text-right"
                      title={new Date(agent.started_at).toLocaleString()}
                    >
                      {timeAgo(agent.started_at)}
                    </span>

                    {/* Actions */}
                    <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 text-yellow-400"
                        onClick={(e) => {
                          e.stopPropagation();
                          setNudgeId(agent.id);
                        }}
                        title="Nudge"
                      >
                        <Zap size={13} />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 text-red-400"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleKill(agent.id);
                        }}
                        disabled={killingId === agent.id}
                        title="Kill"
                      >
                        {killingId === agent.id ? (
                          <Loader2 size={13} className="animate-spin" />
                        ) : (
                          <Skull size={13} />
                        )}
                      </Button>
                    </div>
                  </div>
                ))}
              </Card>
            )}
          </div>
        );
      })}

      {mergedAgents.length === 0 && (
        <div className="text-center py-16">
          <Monitor size={40} className="mx-auto text-border mb-3" />
          <p className="text-muted-foreground font-medium mb-1">
            No agents running
          </p>
          <p className="text-muted-foreground/60 text-sm">
            Create a goal and dispatch tasks to get started
          </p>
        </div>
      )}

      {nudgeId && (
        <NudgeDialog agentId={nudgeId} onClose={() => setNudgeId(null)} />
      )}
    </div>
  );
}
