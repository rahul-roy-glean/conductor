import { useState } from "react";
import type { Project, GoalSettings } from "@/types";
import { updateProject } from "@/api/client";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { useToast } from "@/components/ToastProvider";
import { Loader2 } from "lucide-react";

interface ProjectSettingsProps {
  project: Project;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved?: () => void;
}

export default function ProjectSettings({
  project,
  open,
  onOpenChange,
  onSaved,
}: ProjectSettingsProps) {
  const [form, setForm] = useState<GoalSettings>(project.settings ?? {});
  const [saving, setSaving] = useState(false);
  const { addToast } = useToast();

  const handleSave = async () => {
    setSaving(true);
    try {
      await updateProject(project.id, { settings: form });
      addToast("success", "Project settings saved");
      onOpenChange(false);
      onSaved?.();
    } catch {
      addToast("error", "Failed to save settings");
    } finally {
      setSaving(false);
    }
  };

  const handleToolToggle = (tool: string) => {
    const current = form.allowed_tools ?? [];
    const next = current.includes(tool)
      ? current.filter((t) => t !== tool)
      : [...current, tool];
    setForm({ ...form, allowed_tools: next });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Settings: {project.display_name}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          <div>
            <label className="block text-xs text-muted-foreground mb-1">
              Default Model
            </label>
            <Input
              list="project-model-options"
              value={form.model ?? ""}
              onChange={(e) =>
                setForm({ ...form, model: e.target.value || undefined })
              }
              placeholder="Default (sonnet)"
            />
            <datalist id="project-model-options">
              <option value="claude-opus-4-6[1m]" />
              <option value="claude-opus-4-6" />
              <option value="claude-sonnet-4-5-20250929" />
              <option value="sonnet" />
              <option value="opus" />
              <option value="haiku" />
            </datalist>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-xs text-muted-foreground mb-1">
                Max Budget (USD)
              </label>
              <Input
                type="number"
                step="0.01"
                min="0"
                value={form.max_budget_usd ?? ""}
                onChange={(e) =>
                  setForm({
                    ...form,
                    max_budget_usd: e.target.value
                      ? parseFloat(e.target.value)
                      : undefined,
                  })
                }
                placeholder="5.00"
              />
            </div>
            <div>
              <label className="block text-xs text-muted-foreground mb-1">
                Max Turns
              </label>
              <Input
                type="number"
                min="1"
                value={form.max_turns ?? ""}
                onChange={(e) =>
                  setForm({
                    ...form,
                    max_turns: e.target.value
                      ? parseInt(e.target.value)
                      : undefined,
                  })
                }
                placeholder="50"
              />
            </div>
          </div>

          <div>
            <label className="block text-xs text-muted-foreground mb-1">
              Permission Mode
            </label>
            <select
              value={form.permission_mode ?? ""}
              onChange={(e) =>
                setForm({
                  ...form,
                  permission_mode: e.target.value || undefined,
                })
              }
              className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              <option value="">Default</option>
              <option value="default">Default</option>
              <option value="acceptEdits">Accept Edits</option>
              <option value="bypassPermissions">Bypass Permissions</option>
            </select>
          </div>

          <div>
            <label className="block text-xs text-muted-foreground mb-2">
              Allowed Tools
            </label>
            <div className="grid grid-cols-3 gap-2">
              {[
                "Bash",
                "Read",
                "Edit",
                "Write",
                "Grep",
                "Glob",
                "WebFetch",
                "WebSearch",
                "NotebookEdit",
              ].map((tool) => (
                <label
                  key={tool}
                  className="flex items-center gap-2 text-sm text-foreground cursor-pointer"
                >
                  <input
                    type="checkbox"
                    checked={(form.allowed_tools ?? []).includes(tool)}
                    onChange={() => handleToolToggle(tool)}
                    className="rounded border-input"
                  />
                  {tool}
                </label>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-xs text-muted-foreground mb-1">
              System Prompt
            </label>
            <Textarea
              value={form.system_prompt ?? ""}
              onChange={(e) =>
                setForm({
                  ...form,
                  system_prompt: e.target.value || undefined,
                })
              }
              placeholder="Additional instructions for agents in this project"
              className="h-24"
            />
          </div>
        </div>

        <div className="flex justify-end gap-2 pt-2">
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={saving}>
            {saving && <Loader2 size={14} className="animate-spin" />}
            Save Settings
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
