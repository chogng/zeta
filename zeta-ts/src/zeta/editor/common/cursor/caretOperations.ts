import { getTextGraphemeBoundaries } from "../core/textSegmentation.js";
import { TextPosition } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";

/** Returns the previous grapheme-aware caret position, crossing a line when necessary. */
export function previousCaretPosition(model: TextModel, position: TextPosition): TextPosition {
	model.offsetAt(position);
	if (position.columnIndex > 0) {
		return TextPosition.at(position.lineIndex, previousBoundary(getTextGraphemeBoundaries(model.getLineContent(position.lineIndex)), position.columnIndex));
	}
	if (position.lineIndex === 0) return position;
	const lineIndex = position.lineIndex - 1;
	return TextPosition.at(lineIndex, model.getLineContent(lineIndex).length);
}

/** Returns the next grapheme-aware caret position, crossing a line when necessary. */
export function nextCaretPosition(model: TextModel, position: TextPosition): TextPosition {
	model.offsetAt(position);
	const line = model.getLineContent(position.lineIndex);
	if (position.columnIndex < line.length) {
		return TextPosition.at(position.lineIndex, nextBoundary(getTextGraphemeBoundaries(line), position.columnIndex));
	}
	return position.lineIndex + 1 < model.lineCount ? TextPosition.at(position.lineIndex + 1, 0) : position;
}

/** Returns the first UTF-16 boundary on the current line. */
export function lineStartPosition(model: TextModel, position: TextPosition): TextPosition {
	model.offsetAt(position);
	return TextPosition.at(position.lineIndex, 0);
}

/** Returns the last UTF-16 boundary on the current line. */
export function lineEndPosition(model: TextModel, position: TextPosition): TextPosition {
	model.offsetAt(position);
	return TextPosition.at(position.lineIndex, model.getLineContent(position.lineIndex).length);
}

function previousBoundary(boundaries: readonly number[], column: number): number {
	for (let index = boundaries.length - 1; index >= 0; index -= 1) {
		if (boundaries[index]! < column) return boundaries[index]!;
	}
	return 0;
}

function nextBoundary(boundaries: readonly number[], column: number): number {
	return boundaries.find(boundary => boundary > column) ?? boundaries[boundaries.length - 1]!;
}
