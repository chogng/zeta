import { type TextEdit } from '../languages.js';


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

/** Removes duplicate result selections while preserving the primary selection identity. */
export function normalizeEditorSelections(selections: readonly TextSelectionOffsets[], primaryIndex: number): { readonly selections: readonly TextSelectionOffsets[]; readonly primaryIndex: number } {
	const normalized: TextSelectionOffsets[] = [];
	const sourceIndexes: number[] = [];
	for (const selection of selections) {
		let targetIndex = normalized.findIndex(candidate => candidate.anchorOffset === selection.anchorOffset && candidate.activeOffset === selection.activeOffset);
		if (targetIndex < 0) {
			targetIndex = normalized.length;
			normalized.push(selection);
		}
		sourceIndexes.push(targetIndex);
	}
	return {
		selections: Object.freeze(normalized),
		primaryIndex: sourceIndexes[primaryIndex]!,
	};
}
