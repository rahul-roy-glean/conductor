import { useState, useRef } from "react";
import { Send, Loader2 } from "lucide-react";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";

interface ChatInputProps {
  onSend: (message: string) => void;
  disabled?: boolean;
}

export default function ChatInput({ onSend, disabled }: ChatInputProps) {
  const [value, setValue] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleSend = () => {
    const trimmed = value.trim();
    if (!trimmed || disabled) return;
    onSend(trimmed);
    setValue("");
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="border border-border rounded-lg bg-card p-1.5 focus-within:ring-1 focus-within:ring-ring transition-shadow">
      <Textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Describe your goal or ask a question..."
        className="min-h-[44px] max-h-40 resize-none border-0 shadow-none focus-visible:ring-0 bg-transparent"
        disabled={disabled}
        rows={1}
      />
      <div className="flex items-center justify-between px-1 pt-1">
        <span className="text-xs text-muted-foreground/50">
          Enter to send, Shift+Enter for newline
        </span>
        <Button
          onClick={handleSend}
          disabled={disabled || !value.trim()}
          size="sm"
          className="h-7 px-3"
        >
          {disabled ? (
            <Loader2 size={14} className="animate-spin" />
          ) : (
            <Send size={14} />
          )}
        </Button>
      </div>
    </div>
  );
}
