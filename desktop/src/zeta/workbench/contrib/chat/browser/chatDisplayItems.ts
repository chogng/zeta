import type {
  ThreadItem,
} from "../../../../../../generated/app-server/types.js";

/** Render-ready projection of one committed or transient Thread item. */
export interface IChatDisplayItem {
  readonly id: string;
  readonly type: ThreadItem["type"];
  readonly text: string;
  readonly transient: boolean;
  readonly isError?: boolean;
}

/** Maps a protocol Thread item without interpreting untrusted content. */
export function chatDisplayItem(
  item: ThreadItem,
  transient = false,
): IChatDisplayItem {
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
