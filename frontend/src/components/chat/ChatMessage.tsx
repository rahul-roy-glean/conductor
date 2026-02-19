import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { GoalMessage } from "@/types";
import { cn } from "@/lib/utils";
import { User, Bot } from "lucide-react";

interface ChatMessageProps {
  message: GoalMessage;
  children?: React.ReactNode;
}

export default function ChatMessage({ message, children }: ChatMessageProps) {
  if (message.role === "system") {
    return (
      <div className="flex justify-center py-3">
        <span className="text-xs text-muted-foreground/60 italic px-4 py-1 rounded-full bg-muted/30">
          {message.content}
        </span>
      </div>
    );
  }

  const isUser = message.role === "user";

  return (
    <div
      className={cn(
        "flex gap-3 py-4",
        isUser ? "flex-row-reverse" : "flex-row",
      )}
    >
      {/* Avatar */}
      <div
        className={cn(
          "w-7 h-7 rounded-full flex items-center justify-center shrink-0 mt-0.5",
          isUser
            ? "bg-primary/20 text-primary"
            : "bg-muted text-muted-foreground",
        )}
      >
        {isUser ? <User size={14} /> : <Bot size={14} />}
      </div>

      {/* Message bubble */}
      <div
        className={cn(
          "max-w-[85%] rounded-lg px-4 py-3 text-sm leading-relaxed",
          isUser
            ? "bg-primary/10 border border-primary/20 text-foreground"
            : "bg-card border border-border text-foreground",
        )}
      >
        {children ?? (
          <div className="prose prose-invert prose-sm max-w-none [&_pre]:bg-background [&_pre]:border [&_pre]:border-border [&_pre]:rounded-md [&_pre]:p-3 [&_code]:text-foreground [&_code]:text-xs [&_a]:text-primary [&_p]:my-2 [&_ul]:my-2 [&_ol]:my-2 [&_li]:my-0.5 [&_h1]:text-base [&_h2]:text-sm [&_h3]:text-sm">
            <Markdown remarkPlugins={[remarkGfm]}>{message.content}</Markdown>
          </div>
        )}
      </div>
    </div>
  );
}

export function StreamingMessage({ content }: { content: string }) {
  return (
    <div className="flex gap-3 py-4">
      {/* Avatar */}
      <div className="w-7 h-7 rounded-full flex items-center justify-center shrink-0 mt-0.5 bg-muted text-muted-foreground">
        <Bot size={14} />
      </div>

      {/* Message bubble */}
      <div className="max-w-[85%] rounded-lg px-4 py-3 text-sm leading-relaxed bg-card border border-border text-foreground">
        <div className="prose prose-invert prose-sm max-w-none [&_pre]:bg-background [&_pre]:border [&_pre]:border-border [&_pre]:rounded-md [&_pre]:p-3 [&_code]:text-foreground [&_code]:text-xs [&_a]:text-primary [&_p]:my-2 [&_ul]:my-2 [&_ol]:my-2 [&_li]:my-0.5 [&_h1]:text-base [&_h2]:text-sm [&_h3]:text-sm">
          <Markdown remarkPlugins={[remarkGfm]}>{content}</Markdown>
          <span className="inline-block w-1.5 h-4 ml-0.5 bg-primary/60 animate-pulse rounded-sm" />
        </div>
      </div>
    </div>
  );
}
