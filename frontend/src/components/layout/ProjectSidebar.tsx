import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  listProjects,
  listGoals,
  createGoal,
  createProject,
} from "@/api/client";
import type { Project, GoalSpace } from "@/types";
import { useAgentEvents } from "@/hooks/useAgentEvents";
import {
  FolderGit2,
  ChevronDown,
  ChevronRight,
  Plus,
  Loader2,
  Target,
  Settings,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import ProjectSettings from "@/components/settings/ProjectSettings";

const goalStatusDot: Record<GoalSpace["status"], string> = {
  active: "bg-green-500",
  paused: "bg-yellow-500",
  completed: "bg-gray-500",
  archived: "bg-gray-600",
};

export default function ProjectSidebar() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [goals, setGoals] = useState<GoalSpace[]>([]);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [createGoalProject, setCreateGoalProject] = useState<Project | null>(
    null,
  );
  const [showAddProject, setShowAddProject] = useState(false);
  const { agents } = useAgentEvents();
  const navigate = useNavigate();
  const { id: activeGoalId } = useParams<{ id: string }>();

  // New goal form
  const [goalName, setGoalName] = useState("");
  const [goalDesc, setGoalDesc] = useState("");
  const [creatingGoal, setCreatingGoal] = useState(false);

  // New project form
  const [projectPath, setProjectPath] = useState("");
  const [projectName, setProjectName] = useState("");
  const [creatingProject, setCreatingProject] = useState(false);

  // Project settings
  const [settingsProject, setSettingsProject] = useState<Project | null>(null);

  const loadData = () => {
    listProjects()
      .then(setProjects)
      .catch(() => {});
    listGoals()
      .then(setGoals)
      .catch(() => {});
  };

  useEffect(() => {
    loadData();
  }, []);

  // Count active agents per goal
  const agentsByGoal = new Map<string, number>();
  for (const [, agent] of agents) {
    if (agent.status === "running" || agent.status === "spawning") {
      agentsByGoal.set(
        agent.goal_space_id,
        (agentsByGoal.get(agent.goal_space_id) ?? 0) + 1,
      );
    }
  }

  // Group goals by project (matched by repo_path === project.path)
  const goalsByProject = new Map<string, GoalSpace[]>();
  const unmatchedGoals: GoalSpace[] = [];

  for (const goal of goals) {
    const project = projects.find((p) => p.path === goal.repo_path);
    if (project) {
      if (!goalsByProject.has(project.id)) goalsByProject.set(project.id, []);
      goalsByProject.get(project.id)!.push(goal);
    } else {
      unmatchedGoals.push(goal);
    }
  }

  const toggleProject = (projectId: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(projectId)) {
        next.delete(projectId);
      } else {
        next.add(projectId);
      }
      return next;
    });
  };

  const handleCreateGoal = async () => {
    if (!createGoalProject || !goalName.trim()) return;
    setCreatingGoal(true);
    try {
      const newGoal = await createGoal({
        name: goalName,
        description: goalDesc,
        repo_path: createGoalProject.path,
      });
      setGoalName("");
      setGoalDesc("");
      setCreateGoalProject(null);
      loadData();
      navigate(`/goals/${newGoal.id}`);
    } catch {
      // Silently fail — toast should handle this if needed
    } finally {
      setCreatingGoal(false);
    }
  };

  const handleCreateProject = async () => {
    if (!projectPath.trim() || !projectName.trim()) return;
    setCreatingProject(true);
    try {
      await createProject({
        path: projectPath,
        display_name: projectName,
      });
      setProjectPath("");
      setProjectName("");
      setShowAddProject(false);
      loadData();
    } catch {
      // Silently fail
    } finally {
      setCreatingProject(false);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto py-2">
      {/* Projects tree */}
      {projects.map((project) => {
        const projectGoals = goalsByProject.get(project.id) ?? [];
        const isCollapsed = collapsed.has(project.id);
        const activeCount = projectGoals.reduce(
          (sum, g) => sum + (agentsByGoal.get(g.id) ?? 0),
          0,
        );

        return (
          <div key={project.id}>
            {/* Project header */}
            <div className="flex items-center group">
              <button
                onClick={() => toggleProject(project.id)}
                className="flex items-center gap-1.5 flex-1 px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors rounded-sm mx-1"
              >
                {isCollapsed ? (
                  <ChevronRight size={14} className="shrink-0" />
                ) : (
                  <ChevronDown size={14} className="shrink-0" />
                )}
                <FolderGit2 size={14} className="shrink-0" />
                <span className="truncate font-medium">
                  {project.display_name}
                </span>
                {activeCount > 0 && (
                  <span className="ml-auto text-xs bg-green-900/50 text-green-400 px-1.5 rounded-full shrink-0">
                    {activeCount}
                  </span>
                )}
              </button>
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                onClick={() => setSettingsProject(project)}
                title="Project settings"
              >
                <Settings size={12} />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6 mr-1 opacity-0 group-hover:opacity-100 transition-opacity"
                onClick={() => setCreateGoalProject(project)}
                title="Create goal in project"
              >
                <Plus size={12} />
              </Button>
            </div>

            {/* Goal items */}
            {!isCollapsed && (
              <div className="ml-4">
                {projectGoals.map((goal) => {
                  const runningAgents = agentsByGoal.get(goal.id) ?? 0;
                  const isActive = goal.id === activeGoalId;

                  return (
                    <button
                      key={goal.id}
                      onClick={() => navigate(`/goals/${goal.id}`)}
                      className={cn(
                        "flex items-center gap-2 w-full px-3 py-1.5 text-sm rounded-sm mx-1 transition-colors",
                        isActive
                          ? "bg-muted text-foreground"
                          : "text-muted-foreground hover:text-foreground hover:bg-muted/50",
                      )}
                    >
                      <span
                        className={cn(
                          "w-2 h-2 rounded-full shrink-0",
                          goalStatusDot[goal.status],
                        )}
                      />
                      <span className="truncate">{goal.name}</span>
                      {runningAgents > 0 && (
                        <span className="ml-auto text-xs bg-green-900/50 text-green-400 px-1.5 rounded-full shrink-0">
                          {runningAgents}
                        </span>
                      )}
                    </button>
                  );
                })}
                {projectGoals.length === 0 && (
                  <p className="text-xs text-muted-foreground/60 px-3 py-1.5 mx-1">
                    No goals
                  </p>
                )}
              </div>
            )}
          </div>
        );
      })}

      {/* Unmatched goals (goals without a project) */}
      {unmatchedGoals.length > 0 && (
        <div className="mt-2">
          <div className="px-3 py-1.5 mx-1 text-xs font-medium text-muted-foreground uppercase tracking-wider">
            Other Goals
          </div>
          <div className="ml-4">
            {unmatchedGoals.map((goal) => {
              const runningAgents = agentsByGoal.get(goal.id) ?? 0;
              const isActive = goal.id === activeGoalId;

              return (
                <button
                  key={goal.id}
                  onClick={() => navigate(`/goals/${goal.id}`)}
                  className={cn(
                    "flex items-center gap-2 w-full px-3 py-1.5 text-sm rounded-sm mx-1 transition-colors",
                    isActive
                      ? "bg-muted text-foreground"
                      : "text-muted-foreground hover:text-foreground hover:bg-muted/50",
                  )}
                >
                  <span
                    className={cn(
                      "w-2 h-2 rounded-full shrink-0",
                      goalStatusDot[goal.status],
                    )}
                  />
                  <span className="truncate">{goal.name}</span>
                  {runningAgents > 0 && (
                    <span className="ml-auto text-xs bg-green-900/50 text-green-400 px-1.5 rounded-full shrink-0">
                      {runningAgents}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        </div>
      )}

      {/* Add Project button */}
      <div className="mt-3 mx-1">
        <button
          onClick={() => setShowAddProject(true)}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors rounded-sm w-full"
        >
          <Plus size={12} />
          <span>Add Project</span>
        </button>
      </div>

      {/* No projects placeholder */}
      {projects.length === 0 && unmatchedGoals.length === 0 && (
        <div className="px-4 py-8 text-center">
          <Target size={24} className="mx-auto text-border mb-2" />
          <p className="text-xs text-muted-foreground">No projects yet</p>
          <Button
            variant="ghost"
            size="sm"
            className="mt-2 text-xs"
            onClick={() => setShowAddProject(true)}
          >
            <Plus size={12} /> Add Project
          </Button>
        </div>
      )}

      {/* Create Goal Dialog */}
      {createGoalProject && (
        <Dialog
          open
          onOpenChange={(open) => !open && setCreateGoalProject(null)}
        >
          <DialogContent>
            <DialogHeader>
              <DialogTitle>
                New Goal in {createGoalProject.display_name}
              </DialogTitle>
            </DialogHeader>
            <Input
              value={goalName}
              onChange={(e) => setGoalName(e.target.value)}
              placeholder="Goal name"
              autoFocus
            />
            <Textarea
              value={goalDesc}
              onChange={(e) => setGoalDesc(e.target.value)}
              placeholder="Description (optional)"
              className="h-20 resize-none"
            />
            <div className="flex justify-end gap-2">
              <Button
                variant="ghost"
                onClick={() => setCreateGoalProject(null)}
              >
                Cancel
              </Button>
              <Button
                onClick={handleCreateGoal}
                disabled={creatingGoal || !goalName.trim()}
              >
                {creatingGoal && <Loader2 size={14} className="animate-spin" />}
                Create
              </Button>
            </div>
          </DialogContent>
        </Dialog>
      )}

      {/* Add Project Dialog */}
      {showAddProject && (
        <Dialog open onOpenChange={(open) => !open && setShowAddProject(false)}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Add Project</DialogTitle>
            </DialogHeader>
            <Input
              value={projectPath}
              onChange={(e) => setProjectPath(e.target.value)}
              placeholder="Repository path (e.g. /Users/me/myproject)"
              autoFocus
            />
            <Input
              value={projectName}
              onChange={(e) => setProjectName(e.target.value)}
              placeholder="Display name"
            />
            <div className="flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setShowAddProject(false)}>
                Cancel
              </Button>
              <Button
                onClick={handleCreateProject}
                disabled={
                  creatingProject || !projectPath.trim() || !projectName.trim()
                }
              >
                {creatingProject && (
                  <Loader2 size={14} className="animate-spin" />
                )}
                Add
              </Button>
            </div>
          </DialogContent>
        </Dialog>
      )}

      {/* Project settings modal */}
      {settingsProject && (
        <ProjectSettings
          project={settingsProject}
          open={true}
          onOpenChange={(open) => {
            if (!open) setSettingsProject(null);
          }}
          onSaved={loadData}
        />
      )}
    </div>
  );
}
