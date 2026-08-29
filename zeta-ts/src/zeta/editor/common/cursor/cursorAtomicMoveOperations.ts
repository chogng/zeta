import { TextPosition } from '../core/text.js';
import { getTextGraphemeBoundaries } from '../core/textSegmentation.js';
import { type TextModel } from '../model/textModel.js';

export function previousCursorAtomicPosition(model: TextModel, position: TextPosition): TextPosition {
	if (position.columnIndex === 0) {
		if (position.lineIndex === 0) return position;
		const lineIndex = position.lineIndex - 1;
		return TextPosition.at(lineIndex, model.getLineContent(lineIndex).length);
	}
	const boundaries = getTextGraphemeBoundaries(model.getLineContent(position.lineIndex));
	for (let index = boundaries.length - 1; index >= 0; index -= 1) {
		const boundary = boundaries[index]!;
		if (boundary < position.columnIndex) return TextPosition.at(position.lineIndex, boundary);
	}
	return TextPosition.at(position.lineIndex, 0);
}

export function nextCursorAtomicPosition(model: TextModel, position: TextPosition): TextPosition {
	const line = model.getLineContent(position.lineIndex);
	if (position.columnIndex === line.length) {
		return position.lineIndex + 1 < model.lineCount ? TextPosition.at(position.lineIndex + 1, 0) : position;
	}
	const boundary = getTextGraphemeBoundaries(line).find(candidate => candidate > position.columnIndex);
	return TextPosition.at(position.lineIndex, boundary ?? line.length);
}

export function advanceCursorAtomicPositionInLine(model: TextModel, position: TextPosition, count: number): TextPosition {
	let current = position;
	for (let index = 0; index < count; index += 1) {
		const next = nextCursorAtomicPosition(model, current);
		if (next.lineIndex !== position.lineIndex) break;
		current = next;
	}
	return current;
}
