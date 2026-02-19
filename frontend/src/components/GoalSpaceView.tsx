import { useEffect, useRef, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import {
  getGoal,
  listTasks,
  createTask,
  updateGoal,
  deleteGoal,
  decomposeGoal,
  dispatchGoal,
  retryTask,
  retryAllFailed,
  dispatchTask,
} from "@/api/client";
import type { GoalSpace, GoalSettings, Task, OperationUpdate } from "@/types";
import {
  Plus,
  Play,
  Sparkles,
  Loader2,
  Pencil,
  Trash2,
  Pause,
  Play as PlayIcon,
  RotateCcw,
  AlertTriangle,
  Settings,
  ChevronUp,
  Target,
  Zap,
  Skull,
  DollarSign,
} from "lucide-react";
import { useToast } from "@/components/ToastProvider";
import { useAgentEvents } from "@/hooks/useAgentEvents";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import ChatView from "@/components/chat/ChatView";
import NudgeDialog from "@/components/NudgeDialog";

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diff = Math.floor((now - then) / 1000);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

const statusBadge: Record<GoalSpace["status"], string> = {
  active: "bg-green-900 text-green-300",
  paused: "bg-yellow-900 text-yellow-300",
  completed: "bg-muted text-foreground",
  archived: "bg-card text-muted-foreground",
};

const taskStatusColor: Record<Task["status"], string> = {
  pending: "bg-gray-600",
  assigned: "bg-blue-500",
  running: "bg-green-500 animate-pulse",
  done: "bg-gray-500",
  failed: "bg-red-500",
  blocked: "bg-orange-500",
};

const agentStatusDot: Record<string, string> = {
  spawning: "bg-blue-500",
  running: "bg-green-500 animate-pulse",
  stalled: "bg-yellow-500",
  done: "bg-gray-500",
  failed: "bg-red-500",
  killed: "bg-red-700",
};

function TaskDAG({
  tasks,
  onSelectTask,
}: {
  tasks: Task[];
  onSelectTask?: (task: Task) => void;
}) {
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [dragging, setDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });

  if (tasks.length === 0) return null;

  const nodeWidth = 140;
  const nodeHeight = 36;
  const gapX = 180;
  const gapY = 60;

  const depthMap = new Map<string, number>();
  function getDepth(t: Task): number {
    if (depthMap.has(t.id)) return depthMap.get(t.id)!;
    if (t.depends_on.length === 0) {
      depthMap.set(t.id, 0);
      return 0;
    }
    const parentDepths = t.depends_on
      .map((depId) => tasks.find((x) => x.id === depId))
      .filter(Boolean)
      .map((parent) => getDepth(parent!));
    const d = Math.max(0, ...parentDepths) + 1;
    depthMap.set(t.id, d);
    return d;
  }
  tasks.forEach(getDepth);

  const byDepth = new Map<number, Task[]>();
  tasks.forEach((t) => {
    const d = depthMap.get(t.id) ?? 0;
    byDepth.set(d, [...(byDepth.get(d) ?? []), t]);
  });

  const maxDepth = Math.max(...byDepth.keys());
  const maxPerLevel = Math.max(
    ...[...byDepth.values()].map((arr) => arr.length),
  );
  const svgWidth = (maxDepth + 1) * gapX + 40;
  const svgHeight = maxPerLevel * gapY + 40;

  const positions = new Map<string, { x: number; y: number }>();
  for (const [depth, group] of byDepth) {
    group.forEach((t, i) => {
      positions.set(t.id, {
        x: 20 + depth * gapX,
        y: 20 + i * gapY + ((maxPerLevel - group.length) * gapY) / 2,
      });
    });
  }

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    setZoom((z) => Math.min(Math.max(z * delta, 0.3), 3));
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    setDragging(true);
    setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!dragging) return;
    setPan({ x: e.clientX - dragStart.x, y: e.clientY - dragStart.y });
  };

  const handleMouseUp = () => setDragging(false);

  return (
    <div
      className="mb-4 overflow-hidden rounded-lg border border-border bg-background/50 cursor-grab active:cursor-grabbing"
      style={{ maxHeight: 300 }}
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
    >
      <svg
        width="100%"
        height={300}
        viewBox={`0 0 ${svgWidth} ${svgHeight}`}
        style={{
          transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
          transformOrigin: "0 0",
        }}
      >
        {tasks.flatMap((t) =>
          t.depends_on.map((depId) => {
            const from = positions.get(depId);
            const to = positions.get(t.id);
            if (!from || !to) return null;
            return (
              <line
                key={`${depId}-${t.id}`}
                x1={from.x + nodeWidth}
                y1={from.y + nodeHeight / 2}
                x2={to.x}
                y2={to.y + nodeHeight / 2}
                stroke="#4b5563"
                strokeWidth={1.5}
                markerEnd="url(#arrow)"
              />
            );
          }),
        )}
        <defs>
          <marker
            id="arrow"
            viewBox="0 0 10 10"
            refX="10"
            refY="5"
            markerWidth="6"
            markerHeight="6"
            orient="auto-start-reverse"
          >
            <path d="M 0 0 L 10 5 L 0 10 z" fill="#4b5563" />
          </marker>
        </defs>
        {tasks.map((t) => {
          const pos = positions.get(t.id);
          if (!pos) return null;
          const color = taskStatusColor[t.status].split(" ")[0];
          return (
            <g
              key={t.id}
              className="cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                onSelectTask?.(t);
              }}
            >
              <rect
                x={pos.x}
                y={pos.y}
                width={nodeWidth}
                height={nodeHeight}
                rx={6}
                className={`${color} fill-current`}
                opacity={0.3}
                stroke="#6b7280"
                strokeWidth={1}
              />
              <text
                x={pos.x + 8}
                y={pos.y + nodeHeight / 2 + 4}
                className="fill-gray-200 text-[11px]"
                fontFamily="monospace"
              >
                {t.title.length > 16
                  ? t.title.slice(0, 15) + "..."
                  : t.title}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function OperationProgressPanel({
  activeOp,
  logs,
}: {
  activeOp: OperationUpdate;
  logs: string[];
}) {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs.length]);

  return (
    <Card className="p-4 border-purple-700">
      <div className="flex items-center gap-2 mb-3">
        <Loader2 size={16} className="animate-spin text-purple-400 shrink-0" />
        <span className="text-sm font-medium text-foreground">
          {activeOp.operation_type === "decompose"
            ? "Decomposing goal..."
            : "Dispatching agents..."}
        </span>
      </div>
      {logs.length > 0 && (
        <div
          ref={scrollRef}
          className="max-h-48 overflow-y-auto space-y-1 font-mono text-xs text-muted-foreground"
        >
          {logs.map((msg, i) => (
            <div key={i}>{msg}</div>
          ))}
        </div>
      )}
    </Card>
  );
}

function GoalDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { operations, operationLogs, agents } = useAgentEvents();
  const [goal, setGoal] = useState<GoalSpace | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [showAddTask, setShowAddTask] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [newDesc, setNewDesc] = useState("");
  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [statusLoading, setStatusLoading] = useState<string | null>(null);
  const [activeOperationId, setActiveOperationId] = useState<string | null>(
    null,
  );
  const prevOpStatusRef = useRef<string | null>(null);
  const [activeTab, setActiveTab] = useState("chat");
  const [nudgeId, setNudgeId] = useState<string | null>(null);
  const [selectedDagTask, setSelectedDagTask] = useState<Task | null>(null);

  // Settings panel state
  const [showSettings, setShowSettings] = useState(false);
  const [settingsForm, setSettingsForm] = useState<GoalSettings>({});
  const [savingSettings, setSavingSettings] = useState(false);

  const activeOp = activeOperationId
    ? operations.get(activeOperationId)
    : undefined;
  const operationInProgress = activeOp?.status === "running";

  // React to operation completion/failure
  useEffect(() => {
    if (!activeOp) return;
    const prevStatus = prevOpStatusRef.current;
    prevOpStatusRef.current = activeOp.status;
    if (prevStatus === activeOp.status) return;

    if (activeOp.status === "completed") {
      addToast("success", activeOp.message || "Operation completed");
      loadData();
      setActiveOperationId(null);
      prevOpStatusRef.current = null;
    } else if (activeOp.status === "failed") {
      addToast("error", activeOp.message || "Operation failed");
      setActiveOperationId(null);
      prevOpStatusRef.current = null;
    }
  }, [activeOp?.status]);

  const loadData = () => {
    if (!id) return;
    getGoal(id)
      .then(setGoal)
      .catch(() => addToast("error", "Failed to load goal"));
    listTasks(id)
      .then(setTasks)
      .catch(() => addToast("error", "Failed to load tasks"));
  };

  useEffect(() => {
    loadData();
  }, [id]);

  useEffect(() => {
    if (goal) {
      setSettingsForm(goal.settings || {});
    }
  }, [goal]);

  const handleAddTask = async () => {
    if (!id || !newTitle.trim()) return;
    try {
      await createTask(id, { title: newTitle, description: newDesc });
      setNewTitle("");
      setNewDesc("");
      setShowAddTask(false);
      addToast("success", "Task created");
      loadData();
    } catch {
      addToast("error", "Failed to create task");
    }
  };

  const handleRetryTask = async (taskId: string) => {
    try {
      await retryTask(taskId);
      addToast("success", "Task reset to pending — agent will be dispatched");
      loadData();
    } catch {
      addToast("error", "Failed to retry task");
    }
  };

  const handleRetryAllFailed = async () => {
    if (!id) return;
    try {
      const result = await retryAllFailed(id);
      addToast("success", `Retrying ${result.retried} failed task(s)`);
      loadData();
    } catch {
      addToast("error", "Failed to retry tasks");
    }
  };

  const handleDispatchTask = async (taskId: string, taskTitle: string) => {
    try {
      const result = await dispatchTask(taskId);
      setActiveOperationId(result.operation_id);
      prevOpStatusRef.current = "running";
      addToast("success", `Dispatching agent for "${taskTitle}"`);
    } catch {
      addToast("error", "Failed to dispatch task");
    }
  };

  const handleDecompose = async () => {
    if (!id) return;
    try {
      const result = await decomposeGoal(id);
      setActiveOperationId(result.operation_id);
      prevOpStatusRef.current = "running";
    } catch {
      addToast("error", "Failed to decompose goal");
    }
  };

  const handleDispatch = async () => {
    if (!id) return;
    try {
      const result = await dispatchGoal(id);
      setActiveOperationId(result.operation_id);
      prevOpStatusRef.current = "running";
    } catch {
      addToast("error", "Failed to dispatch goal");
    }
  };

  const handleEdit = () => {
    if (!goal) return;
    setEditName(goal.name);
    setEditDescription(goal.description);
    setEditing(true);
  };

  const handleSaveEdit = async () => {
    if (!id || !editName.trim()) return;
    setSaving(true);
    try {
      await updateGoal(id, { name: editName, description: editDescription });
      addToast("success", "Goal updated");
      setEditing(false);
      loadData();
    } catch {
      addToast("error", "Failed to update goal");
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!id) return;
    if (!window.confirm("Are you sure you want to delete this goal?")) return;
    setDeleting(true);
    try {
      await deleteGoal(id);
      addToast("success", "Goal deleted");
      navigate("/goals");
    } catch {
      addToast("error", "Failed to delete goal");
    } finally {
      setDeleting(false);
    }
  };

  const handleStatusChange = async (status: GoalSpace["status"]) => {
    if (!id) return;
    setStatusLoading(status);
    try {
      await updateGoal(id, { status });
      addToast("success", `Goal ${status}`);
      loadData();
    } catch {
      addToast("error", `Failed to set status to ${status}`);
    } finally {
      setStatusLoading(null);
    }
  };

  const handleSaveSettings = async () => {
    if (!id) return;
    setSavingSettings(true);
    try {
      await updateGoal(id, { settings: settingsForm });
      addToast("success", "Settings saved");
      setShowSettings(false);
      loadData();
    } catch {
      addToast("error", "Failed to save settings");
    } finally {
      setSavingSettings(false);
    }
  };

  const handleToolToggle = (tool: string) => {
    const current = settingsForm.allowed_tools || [];
    const newTools = current.includes(tool)
      ? current.filter((t) => t !== tool)
      : [...current, tool];
    setSettingsForm({ ...settingsForm, allowed_tools: newTools });
  };

  // Filter agents for this goal
  const goalAgents = [...agents.values()].filter(
    (a) => a.goal_space_id === id,
  );
  const activeAgentCount = goalAgents.filter(
    (a) => a.status === "running" || a.status === "spawning",
  ).length;

  if (!goal) return <p className="text-muted-foreground p-8">Loading goal...</p>;

  return (
    <div className="flex flex-col h-full">
      {/* Goal header */}
      <div className="shrink-0 pb-4">
        {editing ? (
          <div className="space-y-3">
            <Input
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              className="text-lg font-bold"
            />
            <Textarea
              value={editDescription}
              onChange={(e) => setEditDescription(e.target.value)}
              className="h-20 resize-none"
            />
            <div className="flex gap-2">
              <Button
                onClick={handleSaveEdit}
                disabled={saving || !editName.trim()}
              >
                {saving && <Loader2 size={14} className="animate-spin" />}
                Save
              </Button>
              <Button variant="ghost" onClick={() => setEditing(false)}>
                Cancel
              </Button>
            </div>
          </div>
        ) : (
          <div className="flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-bold text-foreground">{goal.name}</h1>
              <p className="text-muted-foreground mt-1">{goal.description}</p>
              <div className="flex items-center gap-4 mt-2 text-xs text-muted-foreground">
                <span className="font-mono">{goal.repo_path}</span>
                <span title={new Date(goal.created_at).toLocaleString()}>
                  Created {timeAgo(goal.created_at)}
                </span>
                {goal.updated_at !== goal.created_at && (
                  <span title={new Date(goal.updated_at).toLocaleString()}>
                    Updated {timeAgo(goal.updated_at)}
                  </span>
                )}
              </div>
              {/* Settings badges */}
              {goal.settings &&
                (goal.settings.model ||
                  goal.settings.max_budget_usd ||
                  goal.settings.max_turns ||
                  goal.settings.permission_mode ||
                  (goal.settings.allowed_tools &&
                    goal.settings.allowed_tools.length > 0) ||
                  goal.settings.system_prompt) && (
                  <div className="flex flex-wrap items-center gap-2 mt-2">
                    {goal.settings.model && (
                      <Badge variant="outline" className="bg-blue-900/30 text-blue-300 border-blue-800">
                        Model: {goal.settings.model}
                      </Badge>
                    )}
                    {goal.settings.max_budget_usd !== undefined && (
                      <Badge variant="outline" className="bg-green-900/30 text-green-300 border-green-800">
                        Budget: ${goal.settings.max_budget_usd}
                      </Badge>
                    )}
                    {goal.settings.max_turns !== undefined && (
                      <Badge variant="outline" className="bg-purple-900/30 text-purple-300 border-purple-800">
                        Turns: {goal.settings.max_turns}
                      </Badge>
                    )}
                    {goal.settings.permission_mode && (
                      <Badge variant="outline" className="bg-orange-900/30 text-orange-300 border-orange-800">
                        Mode: {goal.settings.permission_mode}
                      </Badge>
                    )}
                    {goal.settings.allowed_tools &&
                      goal.settings.allowed_tools.length > 0 && (
                        <Badge variant="outline" className="bg-cyan-900/30 text-cyan-300 border-cyan-800">
                          Tools: {goal.settings.allowed_tools.length}
                        </Badge>
                      )}
                    {goal.settings.system_prompt && (
                      <Badge variant="outline" className="bg-pink-900/30 text-pink-300 border-pink-800">
                        Custom prompt
                      </Badge>
                    )}
                  </div>
                )}
            </div>
            <div className="flex items-center gap-2 shrink-0">
              <Badge
                variant="outline"
                className={statusBadge[goal.status]}
              >
                {goal.status}
              </Badge>
              {/* Status controls */}
              {goal.status === "active" && (
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => handleStatusChange("paused")}
                  disabled={statusLoading !== null}
                  title="Pause"
                >
                  {statusLoading === "paused" ? (
                    <Loader2 size={14} className="animate-spin" />
                  ) : (
                    <Pause size={14} />
                  )}
                </Button>
              )}
              {goal.status === "paused" && (
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => handleStatusChange("active")}
                  disabled={statusLoading !== null}
                  title="Resume"
                >
                  {statusLoading === "active" ? (
                    <Loader2 size={14} className="animate-spin" />
                  ) : (
                    <PlayIcon size={14} />
                  )}
                </Button>
              )}
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setShowSettings(!showSettings)}
                title="Settings"
              >
                <Settings size={14} />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={handleEdit}
                title="Edit goal"
              >
                <Pencil size={14} />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                className="text-red-400 hover:text-red-300"
                onClick={handleDelete}
                disabled={deleting}
                title="Delete goal"
              >
                {deleting ? (
                  <Loader2 size={14} className="animate-spin" />
                ) : (
                  <Trash2 size={14} />
                )}
              </Button>
            </div>
          </div>
        )}

        {/* Settings Panel */}
        {showSettings && (
          <Card className="p-4 space-y-4 mt-4">
            <div className="flex items-center justify-between mb-2">
              <h3 className="text-sm font-semibold text-foreground">
                Agent Settings
              </h3>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setShowSettings(false)}
              >
                <ChevronUp size={16} />
              </Button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-muted-foreground mb-1">Model</label>
                <Input
                  type="text"
                  list="model-options"
                  value={settingsForm.model || ""}
                  onChange={(e) =>
                    setSettingsForm({
                      ...settingsForm,
                      model: e.target.value || undefined,
                    })
                  }
                  placeholder="Default (or enter custom model ID)"
                />
                <datalist id="model-options">
                  <option value="claude-opus-4-6[1m]">Opus 4.6 (1M context)</option>
                  <option value="claude-opus-4-6">Opus 4.6</option>
                  <option value="claude-sonnet-4-5-20250929">Sonnet 4.5</option>
                  <option value="claude-3-5-haiku-20241022">Haiku 3.5</option>
                  <option value="sonnet">Sonnet (short alias)</option>
                  <option value="opus">Opus (short alias)</option>
                  <option value="haiku">Haiku (short alias)</option>
                </datalist>
              </div>
              <div>
                <label className="block text-xs text-muted-foreground mb-1">Max Budget (USD)</label>
                <Input
                  type="number"
                  step="0.01"
                  min="0"
                  value={settingsForm.max_budget_usd || ""}
                  onChange={(e) =>
                    setSettingsForm({
                      ...settingsForm,
                      max_budget_usd: e.target.value ? parseFloat(e.target.value) : undefined,
                    })
                  }
                  placeholder="No limit"
                />
              </div>
              <div>
                <label className="block text-xs text-muted-foreground mb-1">Max Turns</label>
                <Input
                  type="number"
                  min="1"
                  value={settingsForm.max_turns || ""}
                  onChange={(e) =>
                    setSettingsForm({
                      ...settingsForm,
                      max_turns: e.target.value ? parseInt(e.target.value) : undefined,
                    })
                  }
                  placeholder="No limit"
                />
              </div>
              <div>
                <label className="block text-xs text-muted-foreground mb-1">Permission Mode</label>
                <select
                  value={settingsForm.permission_mode || ""}
                  onChange={(e) =>
                    setSettingsForm({
                      ...settingsForm,
                      permission_mode: e.target.value || undefined,
                    })
                  }
                  className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                >
                  <option value="">Default</option>
                  <option value="default">Default</option>
                  <option value="acceptEdits">Accept Edits</option>
                  <option value="bypassPermissions">Bypass Permissions</option>
                </select>
              </div>
            </div>

            <div>
              <label className="block text-xs text-muted-foreground mb-2">Allowed Tools</label>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
                {["Bash", "Read", "Edit", "Write", "Grep", "Glob", "WebFetch", "WebSearch", "NotebookEdit"].map((tool) => (
                  <label key={tool} className="flex items-center gap-2 text-sm text-foreground cursor-pointer">
                    <input
                      type="checkbox"
                      checked={(settingsForm.allowed_tools || []).includes(tool)}
                      onChange={() => handleToolToggle(tool)}
                      className="rounded border-input bg-background text-primary focus:ring-ring focus:ring-offset-background"
                    />
                    {tool}
                  </label>
                ))}
              </div>
            </div>

            <div>
              <label className="block text-xs text-muted-foreground mb-1">System Prompt (Additional Instructions)</label>
              <Textarea
                value={settingsForm.system_prompt || ""}
                onChange={(e) =>
                  setSettingsForm({
                    ...settingsForm,
                    system_prompt: e.target.value || undefined,
                  })
                }
                placeholder="Optional additional instructions appended to each agent"
                className="h-24 resize-none"
              />
            </div>

            <div className="flex gap-2 pt-2">
              <Button onClick={handleSaveSettings} disabled={savingSettings}>
                {savingSettings && <Loader2 size={14} className="animate-spin" />}
                Save Settings
              </Button>
              <Button variant="ghost" onClick={() => setShowSettings(false)}>
                Cancel
              </Button>
            </div>
          </Card>
        )}
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col min-h-0">
        <TabsList className="shrink-0">
          <TabsTrigger value="chat">Chat</TabsTrigger>
          <TabsTrigger value="tasks">
            Tasks
            {tasks.length > 0 && (
              <span className="ml-1.5 text-xs text-muted-foreground">
                ({tasks.length})
              </span>
            )}
          </TabsTrigger>
          <TabsTrigger value="agents">
            Agents
            {activeAgentCount > 0 && (
              <span className="ml-1.5 text-xs bg-green-900/50 text-green-400 px-1.5 rounded-full">
                {activeAgentCount}
              </span>
            )}
          </TabsTrigger>
        </TabsList>

        {/* Chat Tab */}
        <TabsContent value="chat" className="flex-1 min-h-0">
          <ChatView goalId={id!} onTasksChanged={loadData} />
        </TabsContent>

        {/* Tasks Tab */}
        <TabsContent value="tasks" className="flex-1 overflow-y-auto space-y-4 py-4">
          {/* Operation progress */}
          {activeOp && operationInProgress && (
            <OperationProgressPanel
              activeOp={activeOp}
              logs={operationLogs.get(activeOp.operation_id) ?? []}
            />
          )}

          {/* Failed tasks warning */}
          {tasks.some((t) => t.status === "failed") && (
            <div className="flex items-center gap-3 px-4 py-3 bg-red-900/30 border border-red-800 rounded-lg">
              <AlertTriangle size={16} className="text-red-400 shrink-0" />
              <span className="text-sm text-red-300">
                {tasks.filter((t) => t.status === "failed").length} task
                {tasks.filter((t) => t.status === "failed").length !== 1 ? "s" : ""}{" "}
                failed.
              </span>
              <Button
                variant="outline"
                size="sm"
                className="bg-red-800 hover:bg-red-700 text-red-100 border-red-700 ml-auto shrink-0"
                onClick={handleRetryAllFailed}
              >
                <RotateCcw size={12} /> Retry All Failed
              </Button>
            </div>
          )}

          {/* Action buttons */}
          <div className="flex gap-3">
            <Button
              variant="outline"
              onClick={() => setShowAddTask(true)}
              disabled={operationInProgress}
            >
              <Plus size={14} /> Add Task
            </Button>
            <Button
              className="bg-purple-700 hover:bg-purple-600 text-white"
              onClick={handleDecompose}
              disabled={operationInProgress}
            >
              {operationInProgress && activeOp?.operation_type === "decompose" ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <Sparkles size={14} />
              )}
              Decompose
            </Button>
            <Button
              className="bg-green-700 hover:bg-green-600 text-white"
              onClick={handleDispatch}
              disabled={operationInProgress}
            >
              {operationInProgress && activeOp?.operation_type === "dispatch" ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <Play size={14} />
              )}
              Dispatch All
            </Button>
          </div>

          {showAddTask && (
            <Card className="p-4 space-y-3">
              <Input
                value={newTitle}
                onChange={(e) => setNewTitle(e.target.value)}
                placeholder="Task title"
              />
              <Textarea
                value={newDesc}
                onChange={(e) => setNewDesc(e.target.value)}
                placeholder="Description"
                className="h-20 resize-none"
              />
              <div className="flex gap-2">
                <Button onClick={handleAddTask}>Create</Button>
                <Button variant="ghost" onClick={() => setShowAddTask(false)}>
                  Cancel
                </Button>
              </div>
            </Card>
          )}

          <TaskDAG tasks={tasks} onSelectTask={setSelectedDagTask} />
          {selectedDagTask && (
            <Card className="p-4 space-y-2 mb-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span
                    className={cn(
                      "w-2.5 h-2.5 rounded-full shrink-0",
                      taskStatusColor[selectedDagTask.status],
                    )}
                  />
                  <span className="text-sm font-medium text-foreground">
                    {selectedDagTask.title}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {selectedDagTask.status}
                  </span>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setSelectedDagTask(null)}
                >
                  Close
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                {selectedDagTask.description || "No description"}
              </p>
              <div className="flex gap-2">
                {(selectedDagTask.status === "pending" || selectedDagTask.status === "failed") && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="bg-green-700 hover:bg-green-600 text-green-100 border-green-700"
                    onClick={() => handleDispatchTask(selectedDagTask.id, selectedDagTask.title)}
                    disabled={operationInProgress}
                  >
                    <Play size={11} /> Dispatch
                  </Button>
                )}
                {selectedDagTask.status === "failed" && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="text-yellow-400"
                    onClick={() => handleRetryTask(selectedDagTask.id)}
                  >
                    <RotateCcw size={11} /> Retry
                  </Button>
                )}
              </div>
            </Card>
          )}

          <div>
            <h2 className="text-lg font-semibold text-foreground mb-3">Tasks</h2>
            <div className="space-y-2">
              {tasks.map((task) => (
                <Card
                  key={task.id}
                  className={cn(
                    "p-3 flex items-center gap-3",
                    task.status === "failed" ? "border-red-800/50" : "",
                  )}
                >
                  <span
                    className={cn(
                      "w-2.5 h-2.5 rounded-full shrink-0",
                      taskStatusColor[task.status],
                    )}
                  />
                  <div className="min-w-0 flex-1">
                    <p className="text-sm text-foreground truncate">{task.title}</p>
                    <p className="text-xs text-muted-foreground truncate">
                      {task.description}
                    </p>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <span
                      className="text-xs text-muted-foreground"
                      title={new Date(task.updated_at).toLocaleString()}
                    >
                      {timeAgo(task.updated_at)}
                    </span>
                    <span
                      className={cn(
                        "text-xs",
                        task.status === "failed" ? "text-red-400" : "text-muted-foreground",
                      )}
                    >
                      {task.status}
                    </span>
                    {(task.status === "pending" || task.status === "failed") && (
                      <Button
                        variant="outline"
                        size="sm"
                        className="bg-green-700 hover:bg-green-600 text-green-100 border-green-700"
                        onClick={() => handleDispatchTask(task.id, task.title)}
                        disabled={operationInProgress}
                        title="Dispatch agent for this task"
                      >
                        <Play size={11} /> Dispatch
                      </Button>
                    )}
                    {task.status === "failed" && (
                      <Button
                        variant="outline"
                        size="sm"
                        className="text-yellow-400"
                        onClick={() => handleRetryTask(task.id)}
                        title="Retry this task"
                      >
                        <RotateCcw size={11} /> Retry
                      </Button>
                    )}
                  </div>
                </Card>
              ))}
              {tasks.length === 0 && (
                <p className="text-muted-foreground text-sm">
                  No tasks yet. Use the Chat tab to describe your goal, or add tasks manually.
                </p>
              )}
            </div>
          </div>
        </TabsContent>

        {/* Agents Tab */}
        <TabsContent value="agents" className="flex-1 overflow-y-auto py-4">
          <div className="space-y-2">
            {goalAgents.length === 0 && (
              <p className="text-muted-foreground text-sm">
                No agents running for this goal.
              </p>
            )}
            {goalAgents.map((agent) => (
              <Card key={agent.id} className="p-3 flex items-center gap-3">
                <span
                  className={cn(
                    "w-2.5 h-2.5 rounded-full shrink-0",
                    agentStatusDot[agent.status] ?? "bg-gray-500",
                  )}
                />
                <div className="min-w-0 flex-1">
                  <p className="text-sm text-foreground font-mono truncate">
                    {agent.branch ?? agent.id.slice(0, 12)}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {agent.status} &middot; {agent.model}
                  </p>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  <span className="text-xs text-muted-foreground font-mono flex items-center gap-1">
                    <DollarSign size={10} />
                    {agent.cost_usd.toFixed(2)}
                  </span>
                  {(agent.status === "running" || agent.status === "stalled") && (
                    <>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 text-yellow-400"
                        onClick={() => setNudgeId(agent.id)}
                        title="Nudge"
                      >
                        <Zap size={13} />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 text-red-400"
                        onClick={() => navigate(`/agents/${agent.id}`)}
                        title="View agent"
                      >
                        <Skull size={13} />
                      </Button>
                    </>
                  )}
                  {(agent.status === "done" || agent.status === "failed") && (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-xs"
                      onClick={() => navigate(`/agents/${agent.id}`)}
                    >
                      View
                    </Button>
                  )}
                </div>
              </Card>
            ))}
          </div>

          {nudgeId && (
            <NudgeDialog agentId={nudgeId} onClose={() => setNudgeId(null)} />
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}

function GoalPlaceholder() {
  return (
    <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
      <Target size={48} className="mb-4 opacity-50" />
      <p className="text-lg font-medium">Select a goal from the sidebar</p>
      <p className="text-sm">Or create a new one using the + button</p>
    </div>
  );
}

export default function GoalSpaceView() {
  const { id } = useParams<{ id: string }>();
  return id ? <GoalDetail /> : <GoalPlaceholder />;
}
