import { useEffect, useRef, useState, useCallback } from "react";
import { listGoalMessages, sendGoalChat } from "@/api/client";
import type { GoalMessage } from "@/types";
import { useAgentEvents } from "@/hooks/useAgentEvents";
import ChatInput from "@/components/chat/ChatInput";
import ChatMessage, { StreamingMessage } from "@/components/chat/ChatMessage";
import TaskProposal from "@/components/chat/TaskProposal";
import { useToast } from "@/components/ToastProvider";

interface ChatViewProps {
  goalId: string;
  onTasksChanged?: () => void;
}

export default function ChatView({ goalId, onTasksChanged }: ChatViewProps) {
  const [messages, setMessages] = useState<GoalMessage[]>([]);
  const [sending, setSending] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const { chatChunks, chatDone, agents } = useAgentEvents();
  const { addToast } = useToast();

  const streamingContent = chatChunks.get(goalId);
  const isDone = chatDone.get(goalId);

  const loadMessages = useCallback(() => {
    listGoalMessages(goalId)
      .then(setMessages)
      .catch(() => {});
  }, [goalId]);

  useEffect(() => {
    loadMessages();
  }, [loadMessages]);

  // Reload messages when streaming completes
  useEffect(() => {
    if (isDone) {
      loadMessages();
      setSending(false);
    }
  }, [isDone, loadMessages]);

  // Auto-scroll on new messages or streaming content
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages.length, streamingContent]);

  // Inject system messages for agent lifecycle events
  const agentSystemMessages: GoalMessage[] = [];
  for (const [, agent] of agents) {
    if (agent.goal_space_id !== goalId) continue;
    if (
      agent.status === "running" &&
      agent.started_at &&
      !messages.some(
        (m) =>
          m.role === "system" &&
          m.content.includes(`Agent started`) &&
          m.content.includes(agent.branch ?? agent.id.slice(0, 8)),
      )
    ) {
      agentSystemMessages.push({
        id: `sys-start-${agent.id}`,
        goal_space_id: goalId,
        role: "system",
        content: `Agent started on task: ${agent.branch ?? agent.id.slice(0, 8)}`,
        message_type: "text",
        metadata_json: "{}",
        created_at: agent.started_at,
      });
    }
    if (
      (agent.status === "done" || agent.status === "failed") &&
      agent.finished_at
    ) {
      const costStr = `$${agent.cost_usd.toFixed(2)}`;
      agentSystemMessages.push({
        id: `sys-end-${agent.id}`,
        goal_space_id: goalId,
        role: "system",
        content:
          agent.status === "done"
            ? `Agent completed task: ${agent.branch ?? agent.id.slice(0, 8)} (${costStr})`
            : `Agent failed on task: ${agent.branch ?? agent.id.slice(0, 8)}`,
        message_type: "text",
        metadata_json: "{}",
        created_at: agent.finished_at,
      });
    }
  }

  // Merge and sort all messages by timestamp
  const allMessages = [...messages, ...agentSystemMessages].sort(
    (a, b) =>
      new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
  );

  const handleSend = async (content: string) => {
    // Optimistically add user message
    const optimistic: GoalMessage = {
      id: `opt-${Date.now()}`,
      goal_space_id: goalId,
      role: "user",
      content,
      message_type: "text",
      metadata_json: "{}",
      created_at: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, optimistic]);
    setSending(true);

    try {
      await sendGoalChat(goalId, content);
    } catch {
      addToast("error", "Failed to send message");
      setSending(false);
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Messages area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto px-4 py-4 space-y-1">
        {allMessages.length === 0 && !streamingContent && (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
            <p className="text-sm">
              Start a conversation to describe your goal
            </p>
          </div>
        )}
        {allMessages.map((msg) => {
          if (
            msg.message_type === "task_proposal" &&
            msg.role === "assistant"
          ) {
            return (
              <ChatMessage key={msg.id} message={msg}>
                <TaskProposal
                  message={msg}
                  goalId={goalId}
                  onTasksCreated={onTasksChanged}
                />
              </ChatMessage>
            );
          }
          return <ChatMessage key={msg.id} message={msg} />;
        })}
        {streamingContent && <StreamingMessage content={streamingContent} />}
      </div>

      {/* Input area */}
      <div className="border-t border-border px-4 py-3">
        <ChatInput
          onSend={handleSend}
          disabled={sending || !!streamingContent}
        />
      </div>
    </div>
  );
}
