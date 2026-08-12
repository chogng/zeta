import { type TextEdit } from "../core/text.js";

export interface TextSelectionOffsets {
  readonly anchorOffset: number;
  readonly activeOffset: number;
}

export enum EditorCommandHistoryMode {
  Isolated = "isolated",
  CoalesceTyping = "coalesceTyping",
  BeginCoalescedTyping = "beginCoalescedTyping",
  CoalesceBackspace = "coalesceBackspace",
  CoalesceDelete = "coalesceDelete",
}

/** Immutable edit plus the selection state that must become active after it commits. */
export interface EditorEditCommand {
  readonly edits: readonly TextEdit[];
  readonly selectionsAfter: readonly TextSelectionOffsets[];
  readonly primarySelectionIndex: number;
  readonly historyMode?: EditorCommandHistoryMode;
}
