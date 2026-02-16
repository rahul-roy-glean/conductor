import { useEffect, useRef, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  listGoals, getGoal, listTasks, createTask, createGoal, updateGoal, deleteGoal, decomposeGoal, dispatchGoal,
  retryTask, retryAllFailed, dispatchTask,
} from '../api/client';
import type { GoalSpace, GoalSettings, Task, OperationUpdate } from '../types';
import { Plus, Play, Sparkles, Loader2, Pencil, Trash2, Pause, Play as PlayIcon, Archive, RotateCcw, AlertTriangle, Settings, ChevronUp, FolderGit2, ChevronDown, ChevronRight } from 'lucide-react';
import { useToast } from './ToastProvider';
import { useAgentEvents } from '../hooks/useAgentEvents';

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diff = Math.floor((now - then) / 1000);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

const statusBadge: Record<GoalSpace['status'], string> = {
  active: 'bg-green-900 text-green-300',
  paused: 'bg-yellow-900 text-yellow-300',
  completed: 'bg-gray-700 text-gray-300',
  archived: 'bg-gray-800 text-gray-500',
};

const taskStatusColor: Record<Task['status'], string> = {
  pending: 'bg-gray-600',
  assigned: 'bg-blue-500',
  running: 'bg-green-500 animate-pulse',
  done: 'bg-gray-500',
  failed: 'bg-red-500',
  blocked: 'bg-orange-500',
};

function GoalList() {
  const [goals, setGoals] = useState<GoalSpace[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [newName, setNewName] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [newRepoPath, setNewRepoPath] = useState('');
  const [creating, setCreating] = useState(false);
  const [collapsedRepos, setCollapsedRepos] = useState<Set<string>>(new Set());
  const navigate = useNavigate();
  const { addToast } = useToast();

  const loadGoals = () => {
    listGoals().then(setGoals).catch(() => addToast('error', 'Failed to load goals'));
  };

  useEffect(() => {
    loadGoals();
  }, []);

  const handleCreate = async () => {
    if (!newName.trim() || !newRepoPath.trim()) return;
    setCreating(true);
    try {
      await createGoal({ name: newName, description: newDescription, repo_path: newRepoPath });
      addToast('success', 'Goal created successfully');
      setNewName('');
      setNewDescription('');
      setNewRepoPath('');
      setShowCreate(false);
      loadGoals();
    } catch {
      addToast('error', 'Failed to create goal');
    } finally {
      setCreating(false);
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

  // Group goals by repo_path
  const groupedGoals = new Map<string, GoalSpace[]>();
  for (const goal of goals) {
    const repo = goal.repo_path;
    if (!groupedGoals.has(repo)) groupedGoals.set(repo, []);
    groupedGoals.get(repo)!.push(goal);
  }

  // Sort groups: active goals first, then by repo path
  const sortedGroups = [...groupedGoals.entries()].sort(([, a], [, b]) => {
    const aActive = a.some((g) => g.status === 'active');
    const bActive = b.some((g) => g.status === 'active');
    if (aActive && !bActive) return -1;
    if (!aActive && bActive) return 1;
    return 0;
  });

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-100">Goal Spaces</h1>
        <button
          onClick={() => setShowCreate(!showCreate)}
          className="flex items-center gap-2 px-3 py-2 text-sm bg-blue-600 hover:bg-blue-500 rounded text-white transition-colors"
        >
          <Plus size={14} /> Create Goal
        </button>
      </div>

      {showCreate && (
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700 space-y-3 mb-6">
          <input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="Goal name (required)"
            className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <textarea
            value={newDescription}
            onChange={(e) => setNewDescription(e.target.value)}
            placeholder="Description"
            className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 h-20 resize-none focus:outline-none focus:border-blue-500"
          />
          <input
            value={newRepoPath}
            onChange={(e) => setNewRepoPath(e.target.value)}
            placeholder="Repository path (required)"
            className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <div className="flex gap-2">
            <button
              onClick={handleCreate}
              disabled={creating || !newName.trim() || !newRepoPath.trim()}
              className="flex items-center gap-2 px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-500 rounded text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {creating && <Loader2 size={14} className="animate-spin" />}
              Create
            </button>
            <button
              onClick={() => setShowCreate(false)}
              className="px-3 py-1.5 text-sm text-gray-400 hover:text-gray-200 transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Goals grouped by repo */}
      {sortedGroups.map(([repoPath, repoGoals]) => {
        const activeCount = repoGoals.filter((g) => g.status === 'active').length;
        const isCollapsed = collapsedRepos.has(repoPath);

        return (
          <div key={repoPath} className="mb-6">
            {/* Repo header - clickable to collapse/expand */}
            <button
              onClick={() => toggleRepo(repoPath)}
              className="flex items-center gap-2 mb-3 px-2 py-1.5 w-full rounded hover:bg-gray-800 transition-colors group"
            >
              {isCollapsed ? (
                <ChevronRight size={14} className="text-gray-500 shrink-0" />
              ) : (
                <ChevronDown size={14} className="text-gray-500 shrink-0" />
              )}
              <FolderGit2 size={14} className="text-gray-500 shrink-0" />
              <span className="text-sm font-mono text-gray-300 truncate">{repoPath}</span>
              <span className="text-xs text-gray-500 shrink-0">
                {repoGoals.length} goal{repoGoals.length !== 1 ? 's' : ''}
                {activeCount > 0 && <span className="text-green-400 ml-1">• {activeCount} active</span>}
              </span>
            </button>

            {/* Goals grid - only show if not collapsed */}
            {!isCollapsed && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {repoGoals.map((goal) => (
                  <div
                    key={goal.id}
                    onClick={() => navigate(`/goals/${goal.id}`)}
                    className="bg-gray-800 rounded-lg p-4 border border-gray-700 hover:border-gray-600 cursor-pointer transition-colors"
                  >
                    <div className="flex items-center justify-between mb-2">
                      <h3 className="font-medium text-gray-100 truncate">{goal.name}</h3>
                      <span className={`text-xs px-2 py-0.5 rounded ${statusBadge[goal.status]}`}>
                        {goal.status}
                      </span>
                    </div>
                    <p className="text-sm text-gray-400 line-clamp-2 mb-2">{goal.description}</p>
                    <div className="flex items-center gap-3 text-xs text-gray-500">
                      <span title={new Date(goal.created_at).toLocaleString()}>
                        Created {timeAgo(goal.created_at)}
                      </span>
                      {goal.updated_at !== goal.created_at && (
                        <span title={new Date(goal.updated_at).toLocaleString()}>
                          Updated {timeAgo(goal.updated_at)}
                        </span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}

      {goals.length === 0 && (
        <p className="text-gray-500 text-center py-12">No goal spaces</p>
      )}
    </div>
  );
}

function TaskDAG({ tasks }: { tasks: Task[] }) {
  if (tasks.length === 0) return null;

  const nodeWidth = 140;
  const nodeHeight = 36;
  const gapX = 180;
  const gapY = 60;

  // Simple layout: group by depth based on dependencies
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
  const maxPerLevel = Math.max(...[...byDepth.values()].map((arr) => arr.length));
  const svgWidth = (maxDepth + 1) * gapX + 40;
  const svgHeight = maxPerLevel * gapY + 40;

  const positions = new Map<string, { x: number; y: number }>();
  for (const [depth, group] of byDepth) {
    group.forEach((t, i) => {
      positions.set(t.id, {
        x: 20 + depth * gapX,
        y: 20 + i * gapY + (maxPerLevel - group.length) * gapY / 2,
      });
    });
  }

  return (
    <svg width={svgWidth} height={svgHeight} className="mb-4">
      {/* Edges */}
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
        <marker id="arrow" viewBox="0 0 10 10" refX="10" refY="5"
          markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" fill="#4b5563" />
        </marker>
      </defs>
      {/* Nodes */}
      {tasks.map((t) => {
        const pos = positions.get(t.id);
        if (!pos) return null;
        const color = taskStatusColor[t.status].split(' ')[0];
        return (
          <g key={t.id}>
            <rect
              x={pos.x} y={pos.y}
              width={nodeWidth} height={nodeHeight}
              rx={6}
              className={`${color} fill-current`}
              opacity={0.3}
              stroke="#6b7280"
              strokeWidth={1}
            />
            <text
              x={pos.x + 8} y={pos.y + nodeHeight / 2 + 4}
              className="fill-gray-200 text-[11px]"
              fontFamily="monospace"
            >
              {t.title.length > 16 ? t.title.slice(0, 15) + '...' : t.title}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

function OperationProgressPanel({ activeOp, logs }: { activeOp: OperationUpdate; logs: string[] }) {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs.length]);

  return (
    <div className="bg-gray-800 rounded-lg p-4 border border-purple-700">
      <div className="flex items-center gap-2 mb-3">
        <Loader2 size={16} className="animate-spin text-purple-400 shrink-0" />
        <span className="text-sm font-medium text-gray-200">
          {activeOp.operation_type === 'decompose' ? 'Decomposing goal...' : 'Dispatching agents...'}
        </span>
      </div>
      {logs.length > 0 && (
        <div
          ref={scrollRef}
          className="max-h-48 overflow-y-auto space-y-1 font-mono text-xs text-gray-400"
        >
          {logs.map((msg, i) => (
            <div key={i}>{msg}</div>
          ))}
        </div>
      )}
    </div>
  );
}

function GoalDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { operations, operationLogs } = useAgentEvents();
  const [goal, setGoal] = useState<GoalSpace | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [showAddTask, setShowAddTask] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [newDesc, setNewDesc] = useState('');
  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState('');
  const [editDescription, setEditDescription] = useState('');
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [statusLoading, setStatusLoading] = useState<string | null>(null);
  const [activeOperationId, setActiveOperationId] = useState<string | null>(null);
  const prevOpStatusRef = useRef<string | null>(null);

  // Settings panel state
  const [showSettings, setShowSettings] = useState(false);
  const [settingsForm, setSettingsForm] = useState<GoalSettings>({});
  const [savingSettings, setSavingSettings] = useState(false);

  const activeOp = activeOperationId ? operations.get(activeOperationId) : undefined;
  const operationInProgress = activeOp?.status === 'running';

  // React to operation completion/failure
  useEffect(() => {
    if (!activeOp) return;
    const prevStatus = prevOpStatusRef.current;
    prevOpStatusRef.current = activeOp.status;
    if (prevStatus === activeOp.status) return;

    if (activeOp.status === 'completed') {
      addToast('success', activeOp.message || 'Operation completed');
      loadData();
      setActiveOperationId(null);
      prevOpStatusRef.current = null;
    } else if (activeOp.status === 'failed') {
      addToast('error', activeOp.message || 'Operation failed');
      setActiveOperationId(null);
      prevOpStatusRef.current = null;
    }
  }, [activeOp?.status]);

  const loadData = () => {
    if (!id) return;
    getGoal(id).then(setGoal).catch(() => addToast('error', 'Failed to load goal'));
    listTasks(id).then(setTasks).catch(() => addToast('error', 'Failed to load tasks'));
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
      setNewTitle('');
      setNewDesc('');
      setShowAddTask(false);
      addToast('success', 'Task created');
      loadData();
    } catch {
      addToast('error', 'Failed to create task');
    }
  };

  const handleRetryTask = async (taskId: string) => {
    try {
      await retryTask(taskId);
      addToast('success', 'Task reset to pending — agent will be dispatched');
      loadData();
    } catch {
      addToast('error', 'Failed to retry task');
    }
  };

  const handleRetryAllFailed = async () => {
    if (!id) return;
    try {
      const result = await retryAllFailed(id);
      addToast('success', `Retrying ${result.retried} failed task(s)`);
      loadData();
    } catch {
      addToast('error', 'Failed to retry tasks');
    }
  };

  const handleDispatchTask = async (taskId: string, taskTitle: string) => {
    try {
      const result = await dispatchTask(taskId);
      setActiveOperationId(result.operation_id);
      prevOpStatusRef.current = 'running';
      addToast('success', `Dispatching agent for "${taskTitle}"`);
    } catch {
      addToast('error', 'Failed to dispatch task');
    }
  };

  const handleDecompose = async () => {
    if (!id) return;
    try {
      const result = await decomposeGoal(id);
      setActiveOperationId(result.operation_id);
      prevOpStatusRef.current = 'running';
    } catch {
      addToast('error', 'Failed to decompose goal');
    }
  };

  const handleDispatch = async () => {
    if (!id) return;
    try {
      const result = await dispatchGoal(id);
      setActiveOperationId(result.operation_id);
      prevOpStatusRef.current = 'running';
    } catch {
      addToast('error', 'Failed to dispatch goal');
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
      addToast('success', 'Goal updated');
      setEditing(false);
      loadData();
    } catch {
      addToast('error', 'Failed to update goal');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!id) return;
    if (!window.confirm('Are you sure you want to delete this goal?')) return;
    setDeleting(true);
    try {
      await deleteGoal(id);
      addToast('success', 'Goal deleted');
      navigate('/goals');
    } catch {
      addToast('error', 'Failed to delete goal');
    } finally {
      setDeleting(false);
    }
  };

  const handleStatusChange = async (status: GoalSpace['status']) => {
    if (!id) return;
    setStatusLoading(status);
    try {
      await updateGoal(id, { status });
      addToast('success', `Goal ${status}`);
      loadData();
    } catch {
      addToast('error', `Failed to set status to ${status}`);
    } finally {
      setStatusLoading(null);
    }
  };

  const handleSaveSettings = async () => {
    if (!id) return;
    setSavingSettings(true);
    try {
      await updateGoal(id, { settings: settingsForm });
      addToast('success', 'Settings saved');
      setShowSettings(false);
      loadData();
    } catch {
      addToast('error', 'Failed to save settings');
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

  if (!goal) return <p className="text-gray-400 p-8">Loading goal...</p>;

  return (
    <div className="space-y-6">
      <div>
        {editing ? (
          <div className="space-y-3">
            <input
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-lg font-bold text-gray-100 focus:outline-none focus:border-blue-500"
            />
            <textarea
              value={editDescription}
              onChange={(e) => setEditDescription(e.target.value)}
              className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 h-20 resize-none focus:outline-none focus:border-blue-500"
            />
            <div className="flex gap-2">
              <button
                onClick={handleSaveEdit}
                disabled={saving || !editName.trim()}
                className="flex items-center gap-2 px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-500 rounded text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {saving && <Loader2 size={14} className="animate-spin" />}
                Save
              </button>
              <button
                onClick={() => setEditing(false)}
                className="px-3 py-1.5 text-sm text-gray-400 hover:text-gray-200 transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        ) : (
          <div className="flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-bold text-gray-100">{goal.name}</h1>
              <p className="text-gray-400 mt-1">{goal.description}</p>
              <div className="flex items-center gap-4 mt-2 text-xs text-gray-500">
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
              {goal.settings && (goal.settings.model || goal.settings.max_budget_usd || goal.settings.max_turns || goal.settings.permission_mode || (goal.settings.allowed_tools && goal.settings.allowed_tools.length > 0) || goal.settings.system_prompt) && (
                <div className="flex flex-wrap items-center gap-2 mt-2">
                  {goal.settings.model && (
                    <span className="text-xs px-2 py-0.5 rounded bg-blue-900/30 text-blue-300 border border-blue-800">
                      Model: {goal.settings.model}
                    </span>
                  )}
                  {goal.settings.max_budget_usd !== undefined && (
                    <span className="text-xs px-2 py-0.5 rounded bg-green-900/30 text-green-300 border border-green-800">
                      Budget: ${goal.settings.max_budget_usd}
                    </span>
                  )}
                  {goal.settings.max_turns !== undefined && (
                    <span className="text-xs px-2 py-0.5 rounded bg-purple-900/30 text-purple-300 border border-purple-800">
                      Turns: {goal.settings.max_turns}
                    </span>
                  )}
                  {goal.settings.permission_mode && (
                    <span className="text-xs px-2 py-0.5 rounded bg-orange-900/30 text-orange-300 border border-orange-800">
                      Mode: {goal.settings.permission_mode}
                    </span>
                  )}
                  {goal.settings.allowed_tools && goal.settings.allowed_tools.length > 0 && (
                    <span className="text-xs px-2 py-0.5 rounded bg-cyan-900/30 text-cyan-300 border border-cyan-800">
                      Tools: {goal.settings.allowed_tools.length}
                    </span>
                  )}
                  {goal.settings.system_prompt && (
                    <span className="text-xs px-2 py-0.5 rounded bg-pink-900/30 text-pink-300 border border-pink-800">
                      Custom prompt
                    </span>
                  )}
                </div>
              )}
            </div>
            <div className="flex items-center gap-2 shrink-0">
              <span className={`text-xs px-2 py-0.5 rounded ${statusBadge[goal.status]}`}>
                {goal.status}
              </span>
              <button
                onClick={() => setShowSettings(!showSettings)}
                className="p-1.5 text-gray-400 hover:text-gray-200 transition-colors"
                title="Settings"
              >
                <Settings size={14} />
              </button>
              <button
                onClick={handleEdit}
                className="p-1.5 text-gray-400 hover:text-gray-200 transition-colors"
                title="Edit goal"
              >
                <Pencil size={14} />
              </button>
              <button
                onClick={handleDelete}
                disabled={deleting}
                className="p-1.5 text-red-400 hover:text-red-300 transition-colors disabled:opacity-50"
                title="Delete goal"
              >
                {deleting ? <Loader2 size={14} className="animate-spin" /> : <Trash2 size={14} />}
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Settings Panel */}
      {showSettings && (
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700 space-y-4">
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-sm font-semibold text-gray-100">Agent Settings</h3>
            <button
              onClick={() => setShowSettings(false)}
              className="text-gray-400 hover:text-gray-200"
            >
              <ChevronUp size={16} />
            </button>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Model */}
            <div>
              <label className="block text-xs text-gray-400 mb-1">Model</label>
              <select
                value={settingsForm.model || ''}
                onChange={(e) => setSettingsForm({ ...settingsForm, model: e.target.value || undefined })}
                className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500"
              >
                <option value="">Default</option>
                <option value="sonnet">Sonnet</option>
                <option value="opus">Opus</option>
                <option value="haiku">Haiku</option>
              </select>
            </div>

            {/* Max Budget */}
            <div>
              <label className="block text-xs text-gray-400 mb-1">Max Budget (USD)</label>
              <input
                type="number"
                step="0.01"
                min="0"
                value={settingsForm.max_budget_usd || ''}
                onChange={(e) => setSettingsForm({ ...settingsForm, max_budget_usd: e.target.value ? parseFloat(e.target.value) : undefined })}
                placeholder="No limit"
                className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500"
              />
            </div>

            {/* Max Turns */}
            <div>
              <label className="block text-xs text-gray-400 mb-1">Max Turns</label>
              <input
                type="number"
                min="1"
                value={settingsForm.max_turns || ''}
                onChange={(e) => setSettingsForm({ ...settingsForm, max_turns: e.target.value ? parseInt(e.target.value) : undefined })}
                placeholder="No limit"
                className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500"
              />
            </div>

            {/* Permission Mode */}
            <div>
              <label className="block text-xs text-gray-400 mb-1">Permission Mode</label>
              <select
                value={settingsForm.permission_mode || ''}
                onChange={(e) => setSettingsForm({ ...settingsForm, permission_mode: e.target.value || undefined })}
                className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500"
              >
                <option value="">Default</option>
                <option value="default">Default</option>
                <option value="acceptEdits">Accept Edits</option>
                <option value="bypassPermissions">Bypass Permissions</option>
              </select>
            </div>
          </div>

          {/* Allowed Tools */}
          <div>
            <label className="block text-xs text-gray-400 mb-2">Allowed Tools</label>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
              {['Bash', 'Read', 'Edit', 'Write', 'Grep', 'Glob', 'WebFetch', 'WebSearch', 'NotebookEdit'].map((tool) => (
                <label key={tool} className="flex items-center gap-2 text-sm text-gray-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={(settingsForm.allowed_tools || []).includes(tool)}
                    onChange={() => handleToolToggle(tool)}
                    className="rounded border-gray-600 bg-gray-900 text-blue-600 focus:ring-blue-500 focus:ring-offset-gray-800"
                  />
                  {tool}
                </label>
              ))}
            </div>
          </div>

          {/* System Prompt */}
          <div>
            <label className="block text-xs text-gray-400 mb-1">System Prompt (Additional Instructions)</label>
            <textarea
              value={settingsForm.system_prompt || ''}
              onChange={(e) => setSettingsForm({ ...settingsForm, system_prompt: e.target.value || undefined })}
              placeholder="Optional additional instructions appended to each agent"
              className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 h-24 resize-none focus:outline-none focus:border-blue-500"
            />
          </div>

          {/* Save Buttons */}
          <div className="flex gap-2 pt-2">
            <button
              onClick={handleSaveSettings}
              disabled={savingSettings}
              className="flex items-center gap-2 px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-500 rounded text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {savingSettings && <Loader2 size={14} className="animate-spin" />}
              Save Settings
            </button>
            <button
              onClick={() => setShowSettings(false)}
              className="px-3 py-1.5 text-sm text-gray-400 hover:text-gray-200 transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Status controls */}
      <div className="flex gap-2">
        {goal.status === 'active' && (
          <button
            onClick={() => handleStatusChange('paused')}
            disabled={statusLoading !== null}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-yellow-800 hover:bg-yellow-700 rounded text-yellow-200 transition-colors disabled:opacity-50"
          >
            {statusLoading === 'paused' ? <Loader2 size={12} className="animate-spin" /> : <Pause size={12} />}
            Pause
          </button>
        )}
        {goal.status === 'paused' && (
          <button
            onClick={() => handleStatusChange('active')}
            disabled={statusLoading !== null}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-green-800 hover:bg-green-700 rounded text-green-200 transition-colors disabled:opacity-50"
          >
            {statusLoading === 'active' ? <Loader2 size={12} className="animate-spin" /> : <PlayIcon size={12} />}
            Resume
          </button>
        )}
        {(goal.status === 'active' || goal.status === 'paused' || goal.status === 'completed') && (
          <button
            onClick={() => handleStatusChange('archived')}
            disabled={statusLoading !== null}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-gray-700 hover:bg-gray-600 rounded text-gray-300 transition-colors disabled:opacity-50"
          >
            {statusLoading === 'archived' ? <Loader2 size={12} className="animate-spin" /> : <Archive size={12} />}
            Archive
          </button>
        )}
      </div>

      {/* Operation progress panel with activity log */}
      {activeOp && operationInProgress && (
        <OperationProgressPanel activeOp={activeOp} logs={operationLogs.get(activeOp.operation_id) ?? []} />
      )}

      {/* Failed tasks warning banner */}
      {tasks.some((t) => t.status === 'failed') && (
        <div className="flex items-center gap-3 px-4 py-3 bg-red-900/30 border border-red-800 rounded-lg">
          <AlertTriangle size={16} className="text-red-400 shrink-0" />
          <span className="text-sm text-red-300">
            {tasks.filter((t) => t.status === 'failed').length} task{tasks.filter((t) => t.status === 'failed').length !== 1 ? 's' : ''} failed.
            {' '}Dependent tasks are blocked until failed tasks are retried.
          </span>
          <button
            onClick={handleRetryAllFailed}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-red-800 hover:bg-red-700 rounded text-red-100 transition-colors ml-auto shrink-0"
          >
            <RotateCcw size={12} /> Retry All Failed
          </button>
        </div>
      )}

      <div className="flex gap-3">
        <button
          onClick={() => setShowAddTask(true)}
          disabled={operationInProgress}
          className="flex items-center gap-2 px-3 py-2 text-sm bg-gray-700 hover:bg-gray-600 rounded text-gray-200 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Plus size={14} /> Add Task
        </button>
        <button
          onClick={handleDecompose}
          disabled={operationInProgress}
          className="flex items-center gap-2 px-3 py-2 text-sm bg-purple-700 hover:bg-purple-600 rounded text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {operationInProgress && activeOp?.operation_type === 'decompose' ? <Loader2 size={14} className="animate-spin" /> : <Sparkles size={14} />}
          Decompose
        </button>
        <button
          onClick={handleDispatch}
          disabled={operationInProgress}
          className="flex items-center gap-2 px-3 py-2 text-sm bg-green-700 hover:bg-green-600 rounded text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {operationInProgress && activeOp?.operation_type === 'dispatch' ? <Loader2 size={14} className="animate-spin" /> : <Play size={14} />}
          Dispatch All
        </button>
      </div>

      {showAddTask && (
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700 space-y-3">
          <input
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
            placeholder="Task title"
            className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500"
          />
          <textarea
            value={newDesc}
            onChange={(e) => setNewDesc(e.target.value)}
            placeholder="Description"
            className="w-full bg-gray-900 border border-gray-600 rounded p-2 text-sm text-gray-100 h-20 resize-none focus:outline-none focus:border-blue-500"
          />
          <div className="flex gap-2">
            <button
              onClick={handleAddTask}
              className="px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-500 rounded text-white transition-colors"
            >
              Create
            </button>
            <button
              onClick={() => setShowAddTask(false)}
              className="px-3 py-1.5 text-sm text-gray-400 hover:text-gray-200 transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      <TaskDAG tasks={tasks} />

      <div>
        <h2 className="text-lg font-semibold text-gray-100 mb-3">Tasks</h2>
        <div className="space-y-2">
          {tasks.map((task) => (
            <div
              key={task.id}
              className={`bg-gray-800 rounded-lg p-3 border flex items-center gap-3 ${
                task.status === 'failed' ? 'border-red-800/50' : 'border-gray-700'
              }`}
            >
              <span className={`w-2.5 h-2.5 rounded-full shrink-0 ${taskStatusColor[task.status]}`} />
              <div className="min-w-0 flex-1">
                <p className="text-sm text-gray-100 truncate">{task.title}</p>
                <p className="text-xs text-gray-500 truncate">{task.description}</p>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <span className="text-xs text-gray-500" title={new Date(task.updated_at).toLocaleString()}>
                  {timeAgo(task.updated_at)}
                </span>
                <span className={`text-xs ${task.status === 'failed' ? 'text-red-400' : 'text-gray-400'}`}>
                  {task.status}
                </span>
                {(task.status === 'pending' || task.status === 'failed') && (
                  <button
                    onClick={() => handleDispatchTask(task.id, task.title)}
                    disabled={operationInProgress}
                    className="flex items-center gap-1 px-2 py-1 text-xs bg-green-700 hover:bg-green-600 rounded text-green-100 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    title="Dispatch agent for this task"
                  >
                    <Play size={11} /> Dispatch
                  </button>
                )}
                {task.status === 'failed' && (
                  <button
                    onClick={() => handleRetryTask(task.id)}
                    className="flex items-center gap-1 px-2 py-1 text-xs bg-gray-700 hover:bg-gray-600 rounded text-yellow-400 transition-colors"
                    title="Retry this task"
                  >
                    <RotateCcw size={11} /> Retry
                  </button>
                )}
              </div>
            </div>
          ))}
          {tasks.length === 0 && (
            <p className="text-gray-500 text-sm">No tasks yet. Decompose the goal or add tasks manually.</p>
          )}
        </div>
      </div>
    </div>
  );
}

export default function GoalSpaceView() {
  const { id } = useParams<{ id: string }>();
  return id ? <GoalDetail /> : <GoalList />;
}
