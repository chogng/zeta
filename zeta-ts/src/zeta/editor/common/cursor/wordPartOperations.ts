import { TextPosition, TextRange } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";

export interface TextWordPartRange { readonly start: number; readonly end: number; }

/** Splits an identifier into editor word parts (camelCase, acronyms, digits, and separators). */
export function getWordPartRanges(text: string): readonly TextWordPartRange[] {
	const ranges: TextWordPartRange[] = [];
	let start = 0;
	const flush = (end: number): void => {
		if (end > start) ranges.push(Object.freeze({ start, end }));
		start = end;
	};
	for (let index = 1; index < text.length; index += 1) {
		const previous = text[index - 1]!;
		const current = text[index]!;
		const next = text[index + 1];
		if (isSeparator(current) !== isSeparator(previous) || isDigit(current) !== isDigit(previous)) {
			flush(index);
			continue;
		}
		if (isLower(previous) && isUpper(current)) {
			flush(index);
			continue;
		}
		if (isUpper(previous) && isUpper(current) && next !== undefined && isLower(next)) flush(index);
	}
	flush(text.length);
	return Object.freeze(ranges);
}

/** Returns the word part containing the position, or the preceding part at end-of-line. */
export function getWordPartRangeAtPosition(model: TextModel, position: TextPosition): TextRange {
	model.offsetAt(position);
	const line = model.getLineContent(position.lineIndex);
	if (line.length === 0) return TextRange.emptyAt(position);
	const probe = position.columnIndex === line.length ? line.length - 1 : position.columnIndex;
	const part = getWordPartRanges(line).find(range => probe >= range.start && probe < range.end);
	if (!part) return TextRange.emptyAt(position);
	return TextRange.from(TextPosition.at(position.lineIndex, part.start), TextPosition.at(position.lineIndex, part.end));
}

function isSeparator(value: string): boolean { return /[^\p{L}\p{N}]/u.test(value); }
function isDigit(value: string): boolean { return /\p{N}/u.test(value); }
function isLower(value: string): boolean { return /\p{Ll}/u.test(value); }
function isUpper(value: string): boolean { return /\p{Lu}/u.test(value); }
