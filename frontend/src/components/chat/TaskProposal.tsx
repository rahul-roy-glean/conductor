import { useState } from "react";
import { createTask, dispatchGoal } from "@/api/client";
import type { GoalMessage } from "@/types";
import { Check, Loader2, RotateCcw, Rocket } from "lucide-react";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ToastProvider";
import { cn } from "@/lib/utils";

interface ProposedTask {
  title: string;
  description: string;
}

interface TaskProposalProps {
  message: GoalMessage;
  goalId: string;
  onTasksCreated?: () => void;
}

export default function TaskProposal({
  message,
  goalId,
  onTasksCreated,
}: TaskProposalProps) {
  const { addToast } = useToast();

  // Parse proposed tasks from metadata
  let proposedTasks: ProposedTask[] = [];
  try {
    const meta = JSON.parse(message.metadata_json);
    proposedTasks = meta.tasks ?? [];
  } catch {
    // If metadata is invalid, show the message content as markdown
  }

  const [tasks, setTasks] = useState<(ProposedTask & { included: boolean })[]>(
    proposedTasks.map((t) => ({ ...t, included: true })),
  );
  const [approved, setApproved] = useState(false);
  const [approving, setApproving] = useState(false);

  const handleToggle = (index: number) => {
    setTasks((prev) =>
      prev.map((t, i) => (i === index ? { ...t, included: !t.included } : t)),
    );
  };

  const handleTitleChange = (index: number, title: string) => {
    setTasks((prev) => prev.map((t, i) => (i === index ? { ...t, title } : t)));
  };

  const handleDescChange = (index: number, description: string) => {
    setTasks((prev) =>
      prev.map((t, i) => (i === index ? { ...t, description } : t)),
    );
  };

  const handleApprove = async () => {
    const included = tasks.filter((t) => t.included);
    if (included.length === 0) return;

    setApproving(true);
    try {
      // Create all tasks
      for (const task of included) {
        await createTask(goalId, {
          title: task.title,
          description: task.description,
        });
      }
      addToast("success", `Created ${included.length} task(s)`);
      setApproved(true);
      onTasksCreated?.();

      // Dispatch agents
      await dispatchGoal(goalId);
      addToast("success", "Dispatching agents...");
    } catch {
      addToast("error", "Failed to create tasks");
    } finally {
      setApproving(false);
    }
  };

  if (proposedTasks.length === 0) {
    // No structured tasks — just render the message content
    return <p className="text-sm text-foreground">{message.content}</p>;
  }

  return (
    <div className="space-y-3">
      <p className="text-sm text-foreground mb-2">{message.content}</p>
      <Card className="divide-y divide-border">
        {tasks.map((task, i) => (
          <div
            key={i}
            className={cn(
              "flex items-start gap-3 p-3 transition-opacity",
              !task.included && "opacity-40",
              approved && "opacity-70",
            )}
          >
            <input
              type="checkbox"
              checked={task.included}
              onChange={() => handleToggle(i)}
              disabled={approved}
              className="mt-1.5 rounded border-input bg-background text-primary focus:ring-ring"
            />
            <div className="flex-1 min-w-0 space-y-1">
              <Input
                value={task.title}
                onChange={(e) => handleTitleChange(i, e.target.value)}
                disabled={approved}
                className="h-7 text-sm font-medium"
              />
              <Input
                value={task.description}
                onChange={(e) => handleDescChange(i, e.target.value)}
                disabled={approved}
                className="h-7 text-xs text-muted-foreground"
              />
            </div>
          </div>
        ))}
      </Card>

      {!approved && (
        <div className="flex gap-2">
          <Button
            onClick={handleApprove}
            disabled={approving || tasks.filter((t) => t.included).length === 0}
            className="bg-green-700 hover:bg-green-600 text-white"
          >
            {approving ? (
              <Loader2 size={14} className="animate-spin" />
            ) : (
              <Rocket size={14} />
            )}
            Approve & Launch ({tasks.filter((t) => t.included).length})
          </Button>
          <Button variant="ghost" size="sm" disabled>
            <RotateCcw size={14} /> Regenerate
          </Button>
        </div>
      )}

      {approved && (
        <div className="flex items-center gap-2 text-sm text-green-400">
          <Check size={14} /> Tasks created and dispatching
        </div>
      )}
    </div>
  );
}
