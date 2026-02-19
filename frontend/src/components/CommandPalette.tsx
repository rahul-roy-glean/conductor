import { useEffect, useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { Command } from "cmdk";
import { listGoals } from "@/api/client";
import type { GoalSpace } from "@/types";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Target, BarChart3, Plus, Cpu } from "lucide-react";

interface CommandPaletteProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function CommandPalette({
  open,
  onOpenChange,
}: CommandPaletteProps) {
  const navigate = useNavigate();
  const [goals, setGoals] = useState<GoalSpace[]>([]);

  useEffect(() => {
    if (open) {
      listGoals()
        .then(setGoals)
        .catch(() => {});
    }
  }, [open]);

  const runAction = useCallback(
    (action: () => void) => {
      onOpenChange(false);
      action();
    },
    [onOpenChange],
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="p-0 gap-0 max-w-lg overflow-hidden"
        aria-describedby={undefined}
      >
        <Command className="bg-popover text-popover-foreground" loop>
          <Command.Input
            placeholder="Search goals, actions..."
            className="h-12 px-4 text-sm bg-transparent border-b border-border outline-none placeholder:text-muted-foreground w-full"
          />
          <Command.List className="max-h-80 overflow-y-auto p-2">
            <Command.Empty className="py-6 text-center text-sm text-muted-foreground">
              No results found.
            </Command.Empty>

            <Command.Group
              heading="Actions"
              className="[&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:text-muted-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5"
            >
              <Command.Item
                onSelect={() => runAction(() => navigate("/goals"))}
                className="flex items-center gap-3 px-3 py-2 rounded text-sm cursor-pointer data-[selected=true]:bg-accent"
              >
                <Plus size={14} className="text-muted-foreground" />
                Create New Goal
              </Command.Item>
              <Command.Item
                onSelect={() => runAction(() => navigate("/"))}
                className="flex items-center gap-3 px-3 py-2 rounded text-sm cursor-pointer data-[selected=true]:bg-accent"
              >
                <Cpu size={14} className="text-muted-foreground" />
                View Fleet
              </Command.Item>
              <Command.Item
                onSelect={() => runAction(() => navigate("/stats"))}
                className="flex items-center gap-3 px-3 py-2 rounded text-sm cursor-pointer data-[selected=true]:bg-accent"
              >
                <BarChart3 size={14} className="text-muted-foreground" />
                View Stats
              </Command.Item>
            </Command.Group>

            {goals.length > 0 && (
              <Command.Group
                heading="Goals"
                className="[&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:text-muted-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5"
              >
                {goals.map((goal) => (
                  <Command.Item
                    key={goal.id}
                    value={`${goal.name} ${goal.description}`}
                    onSelect={() =>
                      runAction(() => navigate(`/goals/${goal.id}`))
                    }
                    className="flex items-center gap-3 px-3 py-2 rounded text-sm cursor-pointer data-[selected=true]:bg-accent"
                  >
                    <Target
                      size={14}
                      className="text-muted-foreground shrink-0"
                    />
                    <div className="min-w-0">
                      <p className="truncate">{goal.name}</p>
                      {goal.description && (
                        <p className="text-xs text-muted-foreground truncate">
                          {goal.description}
                        </p>
                      )}
                    </div>
                  </Command.Item>
                ))}
              </Command.Group>
            )}
          </Command.List>
        </Command>
      </DialogContent>
    </Dialog>
  );
}
