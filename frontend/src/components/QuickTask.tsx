import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  listProjects,
  createGoal,
  createTask,
  dispatchGoal,
} from "@/api/client";
import type { Project } from "@/types";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Loader2, Rocket } from "lucide-react";
import { useToast } from "@/components/ToastProvider";

interface QuickTaskProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function QuickTask({ open, onOpenChange }: QuickTaskProps) {
  const navigate = useNavigate();
  const { addToast } = useToast();
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProject, setSelectedProject] = useState("");
  const [prompt, setPrompt] = useState("");
  const [running, setRunning] = useState(false);

  useEffect(() => {
    if (open) {
      listProjects()
        .then((ps) => {
          setProjects(ps);
          if (ps.length > 0 && !selectedProject) {
            setSelectedProject(ps[0].path);
          }
        })
        .catch(() => {});
    }
  }, [open]);

  const handleRun = async () => {
    if (!prompt.trim() || !selectedProject) return;
    setRunning(true);
    try {
      // Create goal with the prompt as name
      const name = prompt.length > 60 ? prompt.slice(0, 57) + "..." : prompt;
      const goal = await createGoal({
        name,
        description: prompt,
        repo_path: selectedProject,
      });

      // Create a single task matching the prompt
      await createTask(goal.id, {
        title: name,
        description: prompt,
      });

      // Dispatch immediately
      await dispatchGoal(goal.id);

      addToast("success", "Quick task launched");
      onOpenChange(false);
      setPrompt("");
      navigate(`/goals/${goal.id}`);
    } catch {
      addToast("error", "Failed to launch quick task");
    } finally {
      setRunning(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Quick Task</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div>
            <label className="block text-xs text-muted-foreground mb-1">
              Project
            </label>
            <select
              value={selectedProject}
              onChange={(e) => setSelectedProject(e.target.value)}
              className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              {projects.map((p) => (
                <option key={p.id} value={p.path}>
                  {p.display_name}
                </option>
              ))}
              {projects.length === 0 && <option value="">No projects</option>}
            </select>
          </div>
          <div>
            <label className="block text-xs text-muted-foreground mb-1">
              What do you want done?
            </label>
            <Textarea
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="Describe the task..."
              className="h-28 resize-none"
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                  e.preventDefault();
                  handleRun();
                }
              }}
            />
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleRun}
              disabled={running || !prompt.trim() || !selectedProject}
              className="bg-green-700 hover:bg-green-600 text-white"
            >
              {running ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <Rocket size={14} />
              )}
              Run
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
