import { LineRange } from "../ranges/lineRange.js";
import { TextPosition } from "../position.js";
import { TextRange } from "../range.js";
import { AbstractText } from "../text/abstractText.js";
import { StringEdit, StringReplacement } from "./stringEdit.js";
import { TextEdit, TextReplacement } from "./textEdit.js";

/** A line-oriented edit used by diff, folding, and line operations. */
export class LineEdit {
	static readonly empty = new LineEdit([]);
	static deserialize(data: SerializedLineEdit): LineEdit { return new LineEdit(data.map(replacement => LineReplacement.deserialize(replacement))); }
	static fromTextEdit(edit: TextEdit, initialValue: AbstractText): LineEdit {
		const result: LineReplacement[] = [];
		let current: TextReplacement[] = [];
		for (let index = 0; index < edit.replacements.length; index += 1) {
			const replacement = edit.replacements[index]!;
			current.push(replacement);
			const next = edit.replacements[index + 1];
			if (next && next.range.start.lineIndex === replacement.range.end.lineIndex) continue;
			const combined = current.length === 1 ? current[0]! : new TextEdit(current).toReplacement(initialValue);
			result.push(LineReplacement.fromTextReplacement(combined, initialValue));
			current = [];
		}
		return new LineEdit(result);
	}
	static fromStringEdit(edit: StringEdit, initialValue: AbstractText): LineEdit { return LineEdit.fromTextEdit(TextEdit.fromStringEdit(edit, initialValue), initialValue); }
	static createFromUnsorted(edits: readonly LineReplacement[]): LineEdit { return new LineEdit([...edits].sort((left, right) => left.lineRange.startLineIndex - right.lineRange.startLineIndex)); }

	constructor(readonly replacements: readonly LineReplacement[]) {
		for (let index = 1; index < replacements.length; index += 1) if (replacements[index - 1]!.lineRange.endLineIndexExclusive > replacements[index]!.lineRange.startLineIndex) throw new RangeError("Line edits must be sorted and disjoint");
	}

	isEmpty(): boolean { return this.replacements.length === 0; }
	apply(lines: readonly string[]): string[] {
		const result: string[] = [];
		let sourceLine = 0;
		for (const replacement of this.replacements) {
			if (replacement.lineRange.endLineIndexExclusive > lines.length) throw new RangeError("Line edit is outside the text source");
			while (sourceLine < replacement.lineRange.startLineIndex) result.push(lines[sourceLine++]!);
			result.push(...replacement.newLines);
			sourceLine = replacement.lineRange.endLineIndexExclusive;
		}
		while (sourceLine < lines.length) result.push(lines[sourceLine++]!);
		return result;
	}

	inverse(originalLines: readonly string[]): LineEdit {
		const inverse: LineReplacement[] = [];
		let delta = 0;
		for (const replacement of this.replacements) {
			inverse.push(new LineReplacement(new LineRange(replacement.lineRange.startLineIndex + delta, replacement.lineRange.startLineIndex + delta + replacement.newLines.length), originalLines.slice(replacement.lineRange.startLineIndex, replacement.lineRange.endLineIndexExclusive)));
			delta += replacement.newLines.length - replacement.lineRange.length;
		}
		return new LineEdit(inverse);
	}

	toEdit(initialValue: AbstractText): StringEdit { return new StringEdit(this.replacements.map(replacement => replacement.toStringReplacement(initialValue))); }
	toStringEdit(initialValue: AbstractText): StringEdit { return this.toEdit(initialValue); }
	serialize(): SerializedLineEdit { return this.replacements.map(replacement => replacement.serialize()); }
	getNewLineRanges(): LineRange[] { let delta = 0; return this.replacements.map(replacement => { const range = LineRange.ofLength(replacement.lineRange.startLineIndex + delta, replacement.newLines.length); delta += replacement.newLines.length - replacement.lineRange.length; return range; }); }
	mapLineNumber(lineIndex: number): number { let delta = 0; for (const replacement of this.replacements) { if (replacement.lineRange.endLineIndexExclusive > lineIndex) break; delta += replacement.newLines.length - replacement.lineRange.length; } return lineIndex + delta; }
	mapLineRange(range: LineRange): LineRange { return new LineRange(this.mapLineNumber(range.startLineIndex), this.mapLineNumber(range.endLineIndexExclusive)); }
	mapBackLineRange(range: LineRange, originalLines: readonly string[]): LineRange { return this.inverse(originalLines).mapLineRange(range); }
	rebase(base: LineEdit): LineEdit { return new LineEdit(this.replacements.map(replacement => new LineReplacement(base.mapLineRange(replacement.lineRange), replacement.newLines))); }
	touches(other: LineEdit): boolean { return this.replacements.some(left => other.replacements.some(right => left.lineRange.intersectsOrTouches(right.lineRange))); }
	toString(): string { return this.replacements.map(replacement => replacement.toString()).join(","); }
}

export type SerializedLineEdit = SerializedLineReplacement[];

export class LineReplacement {
	constructor(readonly lineRange: LineRange, readonly newLines: readonly string[]) {}

	static deserialize(value: SerializedLineReplacement): LineReplacement { return new LineReplacement(new LineRange(value[0], value[1]), value[2]); }
	static fromSingleTextEdit(edit: TextReplacement, initialValue: AbstractText): LineReplacement { return LineReplacement.fromTextReplacement(edit, initialValue); }

	static fromTextReplacement(edit: TextReplacement, initialValue: AbstractText): LineReplacement {
		const newLines = edit.text.split(/\r\n|\r|\n/);
		const prefix = initialValue.getLineAt(edit.range.start.lineIndex).slice(0, edit.range.start.columnIndex);
		const suffix = initialValue.getLineAt(edit.range.end.lineIndex).slice(edit.range.end.columnIndex);
		newLines[0] = prefix + newLines[0]!;
		newLines[newLines.length - 1] = newLines.at(-1)! + suffix;
		let startLine = edit.range.start.lineIndex;
		let endLineExclusive = edit.range.end.lineIndex + 1;
		if (edit.range.start.columnIndex === initialValue.getLineLength(edit.range.start.lineIndex) && newLines[0] === prefix) {
			startLine += 1;
			newLines.shift();
		}
		if (newLines.length > 0 && startLine < endLineExclusive && edit.range.end.columnIndex === 0 && newLines.at(-1) === suffix) {
			endLineExclusive -= 1;
			newLines.pop();
		}
		return new LineReplacement(new LineRange(startLine, endLineExclusive), newLines);
	}

	toTextReplacement(initialValue: AbstractText): TextReplacement {
		const totalLines = initialValue.length.lineCount + 1;
		if (this.lineRange.endLineIndexExclusive > totalLines) throw new RangeError("Line replacement is outside the text source");
		if (this.lineRange.empty) {
			if (this.newLines.length === 0) return TextReplacement.insert(TextPosition.at(this.lineRange.startLineIndex, 0), "");
			if (this.lineRange.startLineIndex >= totalLines) {
				const lastLine = totalLines - 1;
				return TextReplacement.insert(TextPosition.at(lastLine, initialValue.getLineLength(lastLine)), `\n${this.newLines.join("\n")}`);
			}
			return TextReplacement.insert(TextPosition.at(this.lineRange.startLineIndex, 0), `${this.newLines.join("\n")}\n`);
		}
		if (this.newLines.length === 0) {
			if (this.lineRange.endLineIndexExclusive === totalLines) {
				const start = this.lineRange.startLineIndex === 0
					? TextPosition.at(0, 0)
					: TextPosition.at(this.lineRange.startLineIndex - 1, initialValue.getLineLength(this.lineRange.startLineIndex - 1));
				const end = TextPosition.at(totalLines - 1, initialValue.getLineLength(totalLines - 1));
				return new TextReplacement(TextRange.from(start, end), "");
			}
			return new TextReplacement(TextRange.from(TextPosition.at(this.lineRange.startLineIndex, 0), TextPosition.at(this.lineRange.endLineIndexExclusive, 0)), "");
		}
		if (this.lineRange.endLineIndexExclusive === totalLines) {
			const endLine = totalLines - 1;
			return new TextReplacement(TextRange.from(TextPosition.at(this.lineRange.startLineIndex, 0), TextPosition.at(endLine, initialValue.getLineLength(endLine))), this.newLines.join("\n"));
		}
		return new TextReplacement(TextRange.from(TextPosition.at(this.lineRange.startLineIndex, 0), TextPosition.at(this.lineRange.endLineIndexExclusive, 0)), `${this.newLines.join("\n")}\n`);
	}

	toSingleTextEdit(initialValue: AbstractText): TextReplacement { return this.toTextReplacement(initialValue); }

	toSingleEdit(initialValue: AbstractText): StringReplacement { return this.toStringReplacement(initialValue); }
	toStringReplacement(initialValue: AbstractText) {
		const replacement = this.toTextReplacement(initialValue);
		return new StringReplacement(initialValue.getTransformer().getOffsetRange(replacement.range), replacement.text);
	}

	serialize(): SerializedLineReplacement { return [this.lineRange.startLineIndex, this.lineRange.endLineIndexExclusive, this.newLines]; }
	removeCommonSuffixPrefixLines(initialValue: AbstractText): LineReplacement {
		let startLineIndex = this.lineRange.startLineIndex;
		let endLineIndexExclusive = this.lineRange.endLineIndexExclusive;
		let trimStartCount = 0;
		while (startLineIndex < endLineIndexExclusive && trimStartCount < this.newLines.length && this.newLines[trimStartCount] === initialValue.getLineAt(startLineIndex)) {
			startLineIndex += 1;
			trimStartCount += 1;
		}
		let trimEndCount = 0;
		while (startLineIndex < endLineIndexExclusive && trimStartCount + trimEndCount < this.newLines.length && this.newLines[this.newLines.length - trimEndCount - 1] === initialValue.getLineAt(endLineIndexExclusive - 1)) {
			endLineIndexExclusive -= 1;
			trimEndCount += 1;
		}
		if (trimStartCount === 0 && trimEndCount === 0) return this;
		return new LineReplacement(new LineRange(startLineIndex, endLineIndexExclusive), this.newLines.slice(trimStartCount, this.newLines.length - trimEndCount));
	}
	toLineEdit(): LineEdit { return new LineEdit([this]); }
	toString(): string { return `${this.lineRange.toString()} -> ${JSON.stringify(this.newLines)}`; }
}

export type SerializedLineReplacement = [startLineIndex: number, endLineIndexExclusive: number, newLines: readonly string[]];
