import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { AgentEvent } from "@/types";
import { Terminal } from "lucide-react";

interface TextEventProps {
  event: AgentEvent;
}

export default function TextEvent({ event }: TextEventProps) {
  return (
    <div className="flex items-start gap-3 py-2">
      <Terminal size={14} className="text-green-400 mt-1 shrink-0" />
      <div className="prose prose-invert prose-sm max-w-none [&_pre]:bg-background [&_pre]:border [&_pre]:border-border [&_pre]:rounded [&_code]:text-foreground [&_a]:text-primary">
        <Markdown remarkPlugins={[remarkGfm]}>{event.summary}</Markdown>
      </div>
    </div>
  );
}
