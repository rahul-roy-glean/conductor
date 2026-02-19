import { useState } from "react";
import type { AgentEvent } from "@/types";
import { AlertTriangle, ChevronRight, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

interface ErrorEventProps {
  event: AgentEvent;
}

export default function ErrorEvent({ event }: ErrorEventProps) {
  const [expanded, setExpanded] = useState(false);

  let stackTrace: string | null = null;
  if (event.raw_json) {
    try {
      const parsed = JSON.parse(event.raw_json);
      stackTrace = parsed.stack ?? parsed.stacktrace ?? parsed.trace ?? null;
    } catch {
      // If raw_json is just a string, use it as stack trace
      stackTrace = event.raw_json;
    }
  }

  return (
    <div className="py-1.5">
      <div
        className={cn(
          "rounded border border-red-800/50 bg-red-900/10 px-3 py-2",
        )}
      >
        <button
          onClick={() => stackTrace && setExpanded(!expanded)}
          className="flex items-start gap-3 w-full text-left"
        >
          <AlertTriangle size={14} className="text-red-400 mt-0.5 shrink-0" />
          <span className="text-sm text-red-300 flex-1">{event.summary}</span>
          {stackTrace && (
            <span className="text-red-400 mt-0.5 shrink-0">
              {expanded ? (
                <ChevronDown size={14} />
              ) : (
                <ChevronRight size={14} />
              )}
            </span>
          )}
        </button>
        {expanded && stackTrace && (
          <div className="ml-8 mt-2 p-2 rounded bg-background border border-border font-mono text-xs text-muted-foreground overflow-x-auto max-h-48 overflow-y-auto whitespace-pre-wrap">
            {stackTrace}
          </div>
        )}
      </div>
    </div>
  );
}
