import { commonPrefixLength, commonSuffixLength } from "../../../../base/common/strings.js";
import { TextPosition } from "../position.js";
import { TextRange } from "../range.js";
import { TextLength } from "../text/textLength.js";
import { AbstractText, StringText } from "../text/abstractText.js";
import { StringEdit, StringReplacement } from "./stringEdit.js";

/** A text-coordinate replacement using zero-based UTF-16 positions. */
export class TextReplacement {
	static delete(range: TextRange): TextReplacement { return new TextReplacement(range, ""); }
	static insert(position: TextPosition, text: string): TextReplacement { return new TextReplacement(TextRange.emptyAt(position), text); }
	static fromStringReplacement(replacement: StringReplacement, initialState: AbstractText): TextReplacement { return new TextReplacement(initialState.getTransformer().getRange(replacement.replaceRange), replacement.newText); }

	constructor(readonly range: TextRange, readonly text: string) {}

	get isEmpty(): boolean { return this.range.empty && this.text.length === 0; }
	equals(other: TextReplacement): boolean { return this.range.equals(other.range) && this.text === other.text; }
	toEdit(): TextEdit { return new TextEdit([this]); }

	extendToCoverRange(range: TextRange, initialValue: AbstractText): TextReplacement {
		if (this.range.containsRange(range)) return this;
		const expanded = this.range.plusRange(range);
		const before = initialValue.getValueOfRange(TextRange.from(expanded.start, this.range.start));
		const after = initialValue.getValueOfRange(TextRange.from(this.range.end, expanded.end));
		return new TextReplacement(expanded, before + this.text + after);
	}

	extendToFullLine(initialValue: AbstractText): TextReplacement {
		const endLine = this.range.end.lineIndex;
		return this.extendToCoverRange(TextRange.from(TextPosition.at(this.range.start.lineIndex, 0), TextPosition.at(endLine, initialValue.getLineLength(endLine))), initialValue);
	}

	removeCommonPrefixAndSuffix(text: AbstractText): TextReplacement { return this.removeCommonPrefix(text).removeCommonSuffix(text); }
	removeCommonPrefix(text: AbstractText): TextReplacement {
		const oldText = text.getValueOfRange(this.range);
		const prefixLength = commonPrefixLength(oldText, this.text);
		const start = TextLength.ofText(oldText.slice(0, prefixLength)).addToPosition(this.range.start);
		return new TextReplacement(TextRange.from(start, this.range.end), this.text.slice(prefixLength));
	}
	removeCommonSuffix(text: AbstractText): TextReplacement {
		const oldText = text.getValueOfRange(this.range);
		const suffixLength = commonSuffixLength(oldText, this.text);
		const end = TextLength.ofText(oldText.slice(0, oldText.length - suffixLength)).addToPosition(this.range.start);
		return new TextReplacement(TextRange.from(this.range.start, end), this.text.slice(0, this.text.length - suffixLength));
	}
	isEffectiveDeletion(text: AbstractText): boolean {
		const oldText = text.getValueOfRange(this.range);
		const prefixLength = commonPrefixLength(oldText, this.text);
		const oldRemaining = oldText.slice(prefixLength);
		const newRemaining = this.text.slice(prefixLength);
		const suffixLength = commonSuffixLength(oldRemaining, newRemaining);
		return newRemaining.slice(0, newRemaining.length - suffixLength).length === 0;
	}
	toString(): string { return `${this.range.toString()} -> ${JSON.stringify(this.text)}`; }
}

/** A sorted, disjoint set of text-coordinate replacements. */
export class TextEdit {
	static replace(range: TextRange, text: string): TextEdit { return new TextEdit([new TextReplacement(range, text)]); }
	static delete(range: TextRange): TextEdit { return TextEdit.replace(range, ""); }
	static insert(position: TextPosition, text: string): TextEdit { return TextEdit.replace(TextRange.emptyAt(position), text); }
	static fromStringEdit(edit: StringEdit, initialState: AbstractText): TextEdit { return new TextEdit(edit.replacements.map(replacement => TextReplacement.fromStringReplacement(replacement, initialState))); }
	static fromParallelReplacementsUnsorted(replacements: readonly TextReplacement[]): TextEdit { return new TextEdit([...replacements].sort((left, right) => left.range.start.compareTo(right.range.start))); }

	constructor(readonly replacements: readonly TextReplacement[]) {
		for (let index = 1; index < replacements.length; index += 1) {
			if (replacements[index - 1]!.range.end.isAfter(replacements[index]!.range.start)) throw new RangeError("Text edits must be sorted and disjoint");
		}
	}

	normalize(): TextEdit {
		const normalized: TextReplacement[] = [];
		for (const replacement of this.replacements) {
			const previous = normalized.at(-1);
			if (previous && previous.range.end.equals(replacement.range.start)) normalized[normalized.length - 1] = new TextReplacement(previous.range.plusRange(replacement.range), previous.text + replacement.text);
			else if (!replacement.isEmpty) normalized.push(replacement);
		}
		return new TextEdit(normalized);
	}

	apply(text: AbstractText): string {
		const transformer = text.getTransformer();
		const stringEdit = new StringEdit(this.replacements.map(replacement => new StringReplacement(transformer.getOffsetRange(replacement.range), replacement.text)));
		return stringEdit.apply(text.getValue());
	}

	applyToString(value: string): string { return this.apply(new StringText(value)); }
	toStringEdit(initialState: AbstractText): StringEdit { const transformer = initialState.getTransformer(); return new StringEdit(this.replacements.map(replacement => new StringReplacement(transformer.getOffsetRange(replacement.range), replacement.text))); }
	inverse(document: AbstractText): TextEdit { const newRanges = this.getNewRanges(); return new TextEdit(this.replacements.map((replacement, index) => new TextReplacement(newRanges[index]!, document.getValueOfRange(replacement.range)))); }

	mapPosition(position: TextPosition): TextPosition | TextRange {
		let lineDelta = 0;
		let currentLine = 0;
		let columnDelta = 0;
		for (const replacement of this.replacements) {
			const start = replacement.range.start;
			if (position.isBeforeOrEqual(start)) break;
			const end = replacement.range.end;
			const length = TextLength.ofText(replacement.text);
			const mappedStart = TextPosition.at(start.lineIndex + lineDelta, start.columnIndex + (start.lineIndex + lineDelta === currentLine ? columnDelta : 0));
			if (position.isBefore(end)) return TextRange.from(mappedStart, length.addToPosition(mappedStart));
			const previousColumnDelta = start.lineIndex + lineDelta === currentLine ? columnDelta : 0;
			lineDelta += length.lineCount - (end.lineIndex - start.lineIndex);
			columnDelta = length.lineCount === 0
				? end.lineIndex === start.lineIndex
					? previousColumnDelta + length.columnCount - (end.columnIndex - start.columnIndex)
					: start.columnIndex + previousColumnDelta + length.columnCount - end.columnIndex
				: length.columnCount - end.columnIndex;
			currentLine = end.lineIndex + lineDelta;
		}
		return TextPosition.at(position.lineIndex + lineDelta, position.columnIndex + (position.lineIndex + lineDelta === currentLine ? columnDelta : 0));
	}

	mapRange(range: TextRange): TextRange {
		const start = this.mapPosition(range.start);
		const end = this.mapPosition(range.end);
		return TextRange.from(start instanceof TextPosition ? start : start.start, end instanceof TextPosition ? end : end.end);
	}

	getNewRanges(): TextRange[] {
		return this.replacements.map(replacement => {
			const start = this.mapPosition(replacement.range.start);
			const startPosition = start instanceof TextPosition ? start : start.start;
			return TextRange.from(startPosition, TextLength.ofText(replacement.text).addToPosition(startPosition));
		});
	}

	toReplacement(initialValue: AbstractText): TextReplacement {
		if (this.replacements.length === 0) throw new RangeError("Cannot convert an empty edit to a replacement");
		if (this.replacements.length === 1) return this.replacements[0]!;
		const start = this.replacements[0]!.range.start;
		const end = this.replacements.at(-1)!.range.end;
		let text = "";
		for (let index = 0; index < this.replacements.length; index += 1) {
			const replacement = this.replacements[index]!;
			text += replacement.text;
			if (index + 1 < this.replacements.length) text += initialValue.getValueOfRange(TextRange.from(replacement.range.end, this.replacements[index + 1]!.range.start));
		}
		return new TextReplacement(TextRange.from(start, end), text);
	}

	/** Composes edits through a synthetic source whose line geometry covers both operands. */
	compose(other: TextEdit): TextEdit {
		const source = new StringText(createSyntheticSource(this, other));
		const first = this.toStringEdit(source);
		const afterFirst = new StringText(first.apply(source.value));
		const second = other.toStringEdit(afterFirst);
		return TextEdit.fromStringEdit(first.compose(second), source);
	}

	equals(other: TextEdit): boolean { return this.replacements.length === other.replacements.length && this.replacements.every((replacement, index) => replacement.equals(other.replacements[index]!)); }
}

function createSyntheticSource(first: TextEdit, second: TextEdit): string {
	const replacements = [...first.replacements, ...second.replacements];
	const maxLine = replacements.reduce((max, replacement) => Math.max(max, replacement.range.end.lineIndex), 0);
	const maxColumn = replacements.reduce((max, replacement) => Math.max(max, replacement.range.end.columnIndex), 0);
	const insertedLength = replacements.reduce((sum, replacement) => sum + replacement.text.length, 0);
	const lineCount = maxLine + 2 + replacements.reduce((sum, replacement) => sum + countNewlines(replacement.text), 0);
	const lineLength = Math.max(1, maxColumn + insertedLength + 2);
	return Array.from({ length: lineCount }, () => "x".repeat(lineLength)).join("\n");
}

function countNewlines(value: string): number { let count = 0; for (const character of value) if (character === "\n") count += 1; return count; }
