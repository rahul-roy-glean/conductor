import type { AgentEvent } from "@/types";
import { GitCommit } from "lucide-react";

interface CommitEventProps {
  event: AgentEvent;
}

export default function CommitEvent({ event }: CommitEventProps) {
  let branch: string | null = null;
  let message: string | null = null;

  if (event.raw_json) {
    try {
      const parsed = JSON.parse(event.raw_json);
      branch = parsed.branch ?? null;
      message = parsed.message ?? parsed.commit_message ?? null;
    } catch {
      // ignore
    }
  }

  return (
    <div className="flex items-start gap-3 py-2">
      <GitCommit size={14} className="text-purple-400 mt-0.5 shrink-0" />
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-foreground">Commit</span>
          {branch && (
            <span className="text-xs font-mono bg-purple-900/30 text-purple-300 px-1.5 py-0.5 rounded">
              {branch}
            </span>
          )}
        </div>
        <p className="text-sm text-muted-foreground mt-0.5">
          {message ?? event.summary}
        </p>
      </div>
    </div>
  );
}
