import type { ThreadItem, Turn } from "../../../../services/chat/common/chatService.js";

/** Render-ready projection of one committed or transient Thread item. */
export interface IChatListItem {
  readonly id: string;
  readonly type: ThreadItem["type"] | "turnError";
  readonly text: string;
  readonly transient: boolean;
  readonly isError?: boolean;
}

/** Projects one durable Turn failure as a retryable conversation item. */
export function chatTurnErrorListItem(turn: Turn): IChatListItem | undefined {
  if (turn.status !== "failed") return undefined;
  return {
    id: `turn-error:${turn.turnId}`,
    type: "turnError",
    text: turn.error?.message ?? "Turn failed",
    transient: false,
    isError: true,
  };
}

/** Maps a Chat Thread item without interpreting untrusted content. */
export function chatListItem(item: ThreadItem, transient = false): IChatListItem {
  switch (item.type) {
    case "userMessage":
    case "agentMessage":
    case "reasoning":
    case "plan":
      return {
        id: item.itemId,
        type: item.type,
        text: item.text,
        transient,
      };
    case "userImage":
    case "userImageAttachment":
      return {
        id: item.itemId,
        type: item.type,
        text: "Image",
        transient,
      };
    case "toolCall":
      return {
        id: item.itemId,
        type: item.type,
        text: `${item.name}\n${item.argumentsJson}`,
        transient,
      };
    case "toolResult":
      return {
        id: item.itemId,
        type: item.type,
        text: item.text,
        transient,
        isError: item.isError,
      };
  }
}
