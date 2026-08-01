import type { ThreadItem } from "../../../../services/chat/common/chatService.js";

/** Render-ready projection of one committed or transient Thread item. */
export interface IChatListItem {
  readonly id: string;
  readonly type: ThreadItem["type"];
  readonly text: string;
  readonly transient: boolean;
  readonly isError?: boolean;
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
