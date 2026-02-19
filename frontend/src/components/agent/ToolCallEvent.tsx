import { useState, useEffect, useRef } from "react";
import type { AgentEvent } from "@/types";
import { Wrench, ChevronRight, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import hljs from "highlight.js/lib/core";
import json from "highlight.js/lib/languages/json";
import "highlight.js/styles/github-dark.min.css";

hljs.registerLanguage("json", json);

interface ToolCallEventProps {
  event: AgentEvent;
}

export default function ToolCallEvent({ event }: ToolCallEventProps) {
  const [expanded, setExpanded] = useState(false);
  const codeRef = useRef<HTMLElement>(null);

  let params: Record<string, unknown> | null = null;
  let formattedJson = "";
  if (event.raw_json) {
    try {
      const parsed = JSON.parse(event.raw_json);
      params = parsed.params ?? parsed.input ?? parsed;
      formattedJson = JSON.stringify(parsed, null, 2);
    } catch {
      formattedJson = event.raw_json;
    }
  }

  useEffect(() => {
    if (expanded && codeRef.current) {
      hljs.highlightElement(codeRef.current);
    }
  }, [expanded]);

  const paramsPreview = params
    ? Object.entries(params)
        .slice(0, 3)
        .map(([k, v]) => {
          const val =
            typeof v === "string"
              ? v.slice(0, 60)
              : JSON.stringify(v).slice(0, 60);
          return `${k}: ${val}`;
        })
        .join(", ")
    : null;

  return (
    <div className="py-1.5">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-start gap-3 w-full text-left group hover:bg-accent/30 rounded px-1 -mx-1 py-1"
      >
        <Wrench size={14} className="text-blue-400 mt-0.5 shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-mono font-medium text-foreground">
              {event.tool_name ?? "tool_call"}
            </span>
            {paramsPreview && (
              <span className="text-xs text-muted-foreground truncate">
                {paramsPreview}
              </span>
            )}
          </div>
          {event.summary && event.summary !== event.tool_name && (
            <p className="text-xs text-muted-foreground mt-0.5 truncate">
              {event.summary}
            </p>
          )}
        </div>
        <span className="text-muted-foreground mt-0.5 shrink-0">
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </span>
      </button>
      {expanded && formattedJson && (
        <div
          className={cn(
            "ml-8 mt-1 rounded border border-border bg-background overflow-x-auto max-h-64 overflow-y-auto",
          )}
        >
          <pre className="p-3 m-0">
            <code ref={codeRef} className="language-json text-xs">
              {formattedJson}
            </code>
          </pre>
        </div>
      )}
    </div>
  );
}
