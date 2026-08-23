import { type TextSelection, type TextSelectionSet } from "../../../common/core/selection.js";
import { TextPosition, TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";

export enum EditorEmptySelectionClipboardPolicy {
	Ignore = "ignore",
	Line = "line",
}

export enum EditorClipboardPasteMode {
	Selection = "selection",
	Line = "line",
}

export interface EditorClipboardEntry {
	readonly text: string;
	readonly sourceRange: TextRange;
	readonly pasteMode: EditorClipboardPasteMode;
}

/** Resolves stable clipboard text and cut ranges for every selection. */
export function getEditorClipboardEntries(model: TextModel, selections: TextSelectionSet, emptySelectionPolicy: EditorEmptySelectionClipboardPolicy): readonly EditorClipboardEntry[] {
	assertEmptySelectionPolicy(emptySelectionPolicy);
	return Object.freeze(selections.selections.map(selection =>
		clipboardEntry(model, selection, emptySelectionPolicy)
	));
}

function clipboardEntry(model: TextModel, selection: TextSelection, emptySelectionPolicy: EditorEmptySelectionClipboardPolicy): EditorClipboardEntry {
	if (!selection.collapsed) {
		return Object.freeze({
			text: model.getTextInRange(selection.range),
			sourceRange: selection.range,
			pasteMode: EditorClipboardPasteMode.Selection,
		});
	}
	if (emptySelectionPolicy === EditorEmptySelectionClipboardPolicy.Ignore) {
		return Object.freeze({
			text: "",
			sourceRange: selection.range,
			pasteMode: EditorClipboardPasteMode.Selection,
		});
	}
	const lineIndex = selection.active.lineIndex;
	return Object.freeze({
		text: `${model.getLineContent(lineIndex)}\n`,
		sourceRange: completeLineCutRange(model, lineIndex),
		pasteMode: EditorClipboardPasteMode.Line,
	});
}

function completeLineCutRange(model: TextModel, lineIndex: number): TextRange {
	if (lineIndex + 1 < model.lineCount) {
		return TextRange.from(
			TextPosition.at(lineIndex, 0),
			TextPosition.at(lineIndex + 1, 0),
		);
	}
	if (lineIndex > 0) {
		const previousLineIndex = lineIndex - 1;
		return TextRange.from(
			TextPosition.at(
				previousLineIndex,
				model.getLineContent(previousLineIndex).length,
			),
			TextPosition.at(lineIndex, model.getLineContent(lineIndex).length),
		);
	}
	return TextRange.from(
		TextPosition.at(0, 0),
		TextPosition.at(0, model.getLineContent(0).length),
	);
}

function assertEmptySelectionPolicy(policy: EditorEmptySelectionClipboardPolicy): void {
	if (!Object.values(EditorEmptySelectionClipboardPolicy).includes(policy)) {
		throw new TypeError("Unknown editor empty-selection clipboard policy");
	}
}
