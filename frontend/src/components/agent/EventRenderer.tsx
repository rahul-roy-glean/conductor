import type { AgentEvent } from "@/types";
import ToolCallEvent from "@/components/agent/ToolCallEvent";
import TextEvent from "@/components/agent/TextEvent";
import CommitEvent from "@/components/agent/CommitEvent";
import ErrorEvent from "@/components/agent/ErrorEvent";
import { DollarSign, AlertTriangle, FileText } from "lucide-react";

interface EventRendererProps {
  event: AgentEvent;
}

export default function EventRenderer({ event }: EventRendererProps) {
  switch (event.event_type) {
    case "tool_call":
      return <ToolCallEvent event={event} />;
    case "text_output":
      return <TextEvent event={event} />;
    case "commit":
      return <CommitEvent event={event} />;
    case "error":
    case "stall":
      return <ErrorEvent event={event} />;
    case "tool_result":
      return (
        <div className="flex items-start gap-3 py-1.5">
          <FileText size={14} className="text-muted-foreground mt-0.5 shrink-0" />
          <span className="text-xs text-muted-foreground truncate">{event.summary}</span>
        </div>
      );
    case "cost_update":
      return (
        <div className="flex items-start gap-3 py-1.5">
          <DollarSign size={14} className="text-green-400 mt-0.5 shrink-0" />
          <span className="text-xs text-muted-foreground">
            {event.summary}
            {event.cost_delta_usd !== null && (
              <span className="ml-1 font-mono text-green-400">
                +${event.cost_delta_usd.toFixed(4)}
              </span>
            )}
          </span>
        </div>
      );
    default: {
      const Icon = event.event_type === "stall" ? AlertTriangle : FileText;
      return (
        <div className="flex items-start gap-3 py-1.5">
          <Icon size={14} className="text-muted-foreground mt-0.5 shrink-0" />
          <span className="text-sm text-foreground">{event.summary}</span>
        </div>
      );
    }
  }
}
