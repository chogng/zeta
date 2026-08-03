import { EditorCommandHistoryMode, type EditorEditCommand } from "./editorSelectionController.js";
import { type LanguageBracketMatcher } from "./languageBracketMatcher.js";
import { type TextSelectionSet } from "./selection.js";
import { type TextEdit } from "./text.js";

interface BracketDeletion {
  readonly startOffset: number;
  readonly endOffset: number;
  readonly edit: TextEdit;
}

/** Removes every distinct matched bracket pair containing a collapsed cursor. */
export function createRemoveMatchingBracketsCommand(matcher: LanguageBracketMatcher, selections: TextSelectionSet): EditorEditCommand | undefined {
  const model = matcher.textModel;
  const deletions = new Map<string, BracketDeletion>();
  for (const selection of selections.selections) {
    if (!selection.collapsed) continue;
    const match = matcher.findMatch(selection.active);
    if (!match) continue;
    addDeletion(deletions, model.offsetAt(match.opening.start), model.offsetAt(match.opening.end), match.opening);
    addDeletion(deletions, model.offsetAt(match.closing.start), model.offsetAt(match.closing.end), match.closing);
  }
  if (deletions.size === 0) return undefined;
  const ordered = [...deletions.values()].sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset);
  const selectionsAfter = selections.selections.map(selection => {
    const match = selection.collapsed ? matcher.findMatch(selection.active) : undefined;
    const targetOffset = match ? model.offsetAt(match.opening.start) : model.offsetAt(selection.active);
    const mapped = mapOffsetThroughDeletions(targetOffset, ordered);
    return Object.freeze({
      anchorOffset: selection.collapsed ? mapped : mapOffsetThroughDeletions(model.offsetAt(selection.anchor), ordered),
      activeOffset: mapped,
    });
  });
  const normalizedSelections = normalizeSelectionsAfter(selectionsAfter, selections.primaryIndex);
  return Object.freeze({
    edits: Object.freeze(ordered.map(deletion => deletion.edit)),
    selectionsAfter: normalizedSelections.selections,
    primarySelectionIndex: normalizedSelections.primaryIndex,
    historyMode: EditorCommandHistoryMode.Isolated,
  });
}

function addDeletion(target: Map<string, BracketDeletion>, startOffset: number, endOffset: number, range: TextEdit["range"]): void {
  const key = `${startOffset}:${endOffset}`;
  if (target.has(key)) return;
  target.set(key, Object.freeze({
    startOffset,
    endOffset,
    edit: Object.freeze({ range, text: "" }),
  }));
}

function mapOffsetThroughDeletions(offset: number, deletions: readonly BracketDeletion[]): number {
  let delta = 0;
  for (const deletion of deletions) {
    if (offset < deletion.startOffset) break;
    if (offset <= deletion.endOffset) return deletion.startOffset - delta;
    delta += deletion.endOffset - deletion.startOffset;
  }
  return offset - delta;
}

function normalizeSelectionsAfter(selections: readonly { readonly anchorOffset: number; readonly activeOffset: number }[], primaryIndex: number): {
  readonly selections: readonly { readonly anchorOffset: number; readonly activeOffset: number }[];
  readonly primaryIndex: number;
} {
  const normalized: { readonly anchorOffset: number; readonly activeOffset: number }[] = [];
  const sourceToNormalized: number[] = [];
  for (const selection of selections) {
    let index = normalized.findIndex(candidate =>
      candidate.anchorOffset === selection.anchorOffset && candidate.activeOffset === selection.activeOffset
    );
    if (index < 0) {
      index = normalized.length;
      normalized.push(selection);
    }
    sourceToNormalized.push(index);
  }
  return Object.freeze({
    selections: Object.freeze(normalized),
    primaryIndex: sourceToNormalized[primaryIndex]!,
  });
}
