import { createSelectionEditCommand, type EditorSelectionEdit } from "./cursorTypeEditOperations.js";
import { createTypeTextCommand } from "./cursorTypeOperations.js";
import { EditorCommandHistoryMode, type EditorEditCommand } from "../commands/editorEditCommand.js";
import { type TextSelectionSet } from "../core/selection.js";
import { normalizeTextLineEndings, TextPosition, TextRange } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";
import { getTextGraphemeBoundaries } from "../core/textSegmentation.js";

/** Replaces following graphemes for collapsed selections while overtype is active. */
export function createOvertypeTextCommand(model: TextModel, selections: TextSelectionSet, text: string): EditorEditCommand {
	if (typeof text !== "string") throw new TypeError("Overtype text must be a string");
	const normalized = normalizeTextLineEndings(text);
	if (normalized.includes("\n")) return createTypeTextCommand(model, selections, normalized);
	const graphemeCount = getTextGraphemeBoundaries(normalized).length - 1;
	return createSelectionEditCommand(model, selections, selections.selections.map(selection => {
		const range = selection.collapsed
			? TextRange.from(selection.active, overtypeEnd(model, selection.active, graphemeCount))
			: selection.range;
		return Object.freeze({ range, text: normalized, anchorOffsetInText: normalized.length, activeOffsetInText: normalized.length } satisfies EditorSelectionEdit);
	}), EditorCommandHistoryMode.CoalesceTyping);
}

function overtypeEnd(model: TextModel, position: TextPosition, graphemeCount: number): TextPosition {
	const boundaries = getTextGraphemeBoundaries(model.getLineContent(position.lineIndex));
	let column = position.columnIndex;
	for (let index = 0; index < graphemeCount; index += 1) {
		const next = boundaries.find(boundary => boundary > column);
		if (next === undefined) break;
		column = next;
	}
	return TextPosition.at(position.lineIndex, column);
}
