import { findLastIdxMonotonous } from "../../../../base/common/arraysFind.js";
import { CharCode } from "../../../../base/common/charCode.js";
import { Position } from "../position.js";
import { Range } from "../range.js";
import { OffsetRange } from "../ranges/offsetRange.js";
import { TextLength } from "./textLength.js";
import { StringEdit, StringReplacement } from '../edits/stringEdit.js';
import type { TextEdit, TextReplacement } from '../edits/textEdit.js';

/** Shared coordinate conversion contract for strings and line-based models. */
export abstract class PositionOffsetTransformerBase {
	abstract getOffset(position: Position): number;
	abstract getPosition(offset: number): Position;

	getOffsetRange(range: Range): OffsetRange { return new OffsetRange(this.getOffset(range.getStartPosition()), this.getOffset(range.getEndPosition())); }
	getRange(range: OffsetRange): Range { return Range.fromPositions(this.getPosition(range.start), this.getPosition(range.endExclusive)); }
	getStringEdit(edit: TextEdit): StringEdit { return new Deps.deps.StringEdit(edit.replacements.map(replacement => this.getStringReplacement(replacement))); }
	getStringReplacement(edit: TextReplacement): StringReplacement { return new Deps.deps.StringReplacement(this.getOffsetRange(edit.range), edit.text); }
	getTextReplacement(edit: StringReplacement): TextReplacement { return new Deps.deps.TextReplacement(this.getRange(edit.replaceRange), edit.newText); }
	getTextEdit(edit: StringEdit): TextEdit { return new Deps.deps.TextEdit(edit.replacements.map(replacement => this.getTextReplacement(replacement))); }
}

interface IDeps {
	StringEdit: typeof StringEdit;
	StringReplacement: typeof StringReplacement;
	TextReplacement: typeof TextReplacement;
	TextEdit: typeof TextEdit;
	TextLength: typeof TextLength;
}

class Deps {
	static _deps: IDeps | undefined;
	static get deps(): IDeps {
		if (!this._deps) throw new Error('Position/offset transformer dependencies are not initialized');
		return this._deps;
	}
}

export function _setPositionOffsetTransformerDependencies(deps: IDeps): void { Deps._deps = deps; }

/** Converts one-based editor positions and zero-based UTF-16 offsets in normalized text. */
export class PositionOffsetTransformer extends PositionOffsetTransformerBase {
	private _lineStartOffsetByLineIdx: number[] | undefined;
	private _lineEndOffsetByLineIdx: number[] | undefined;

	constructor(public readonly text: string) {
		super();
	}

	private get lineStartOffsetByLineIdx(): number[] { this._computeLineOffsets(); return this._lineStartOffsetByLineIdx!; }
	private get lineEndOffsetByLineIdx(): number[] { this._computeLineOffsets(); return this._lineEndOffsetByLineIdx!; }
	private _computeLineOffsets(): void {
		if (this._lineStartOffsetByLineIdx) return;
		this._lineStartOffsetByLineIdx = [0];
		this._lineEndOffsetByLineIdx = [];
		for (let index = 0; index < this.text.length; index += 1) {
			if (this.text.charCodeAt(index) !== CharCode.LineFeed) continue;
			this._lineStartOffsetByLineIdx.push(index + 1);
			this._lineEndOffsetByLineIdx.push(index > 0 && this.text.charCodeAt(index - 1) === CharCode.CarriageReturn ? index - 1 : index);
		}
		this._lineEndOffsetByLineIdx.push(this.text.length);
	}

	get textLength(): TextLength {
		const lastLine = this.lineStartOffsetByLineIdx.length - 1;
		return new TextLength(lastLine, this.text.length - this.lineStartOffsetByLineIdx[lastLine]!);
	}

	getOffset(position: Position): number {
		const validated = this._validatePosition(position);
		return this.lineStartOffsetByLineIdx[validated.lineNumber - 1]! + validated.column - 1;
	}

	private _validatePosition(position: Position): Position {
		if (position.lineNumber < 1) return new Position(1, 1);
		const lineCount = this.textLength.lineCount + 1;
		if (position.lineNumber > lineCount) return new Position(lineCount, this.getLineLength(lineCount) + 1);
		if (position.column < 1) return new Position(position.lineNumber, 1);
		const lineLength = this.getLineLength(position.lineNumber);
		return position.column - 1 > lineLength ? new Position(position.lineNumber, lineLength + 1) : position;
	}

	getPosition(offset: number): Position {
		const lineIndex = findLastIdxMonotonous(this.lineStartOffsetByLineIdx, start => start <= offset);
		return new Position(lineIndex + 1, offset - this.lineStartOffsetByLineIdx[lineIndex]! + 1);
	}

	getTextLength(range: OffsetRange): TextLength { return TextLength.ofRange(this.getRange(range)); }

	getLineLength(lineNumber: number): number {
		return this.lineEndOffsetByLineIdx[lineNumber - 1]! - this.lineStartOffsetByLineIdx[lineNumber - 1]!;
	}
}

/** A transformer backed by a model that exposes logical line contents. */
export class LineBasedPositionOffsetTransformer extends PositionOffsetTransformerBase {
	private readonly lineStarts: readonly number[];
	private readonly lineLengths: readonly number[];

	constructor(lines: readonly string[]) {
		super();
		if (lines.length === 0) throw new RangeError("A text source must contain at least one line");
		const starts: number[] = [];
		const lengths: number[] = [];
		let offset = 0;
		for (const line of lines) {
			starts.push(offset);
			lengths.push(line.length);
			offset += line.length + 1;
		}
		this.lineStarts = starts;
		this.lineLengths = lengths;
	}

	get textLength(): TextLength {
		const lastLineIndex = this.lineLengths.length - 1;
		return new TextLength(lastLineIndex, this.lineLengths[lastLineIndex]);
	}

	getOffset(position: Position): number {
		const lineIndex = Math.min(Math.max(position.lineNumber - 1, 0), this.lineLengths.length - 1);
		const columnIndex = Math.min(Math.max(position.column - 1, 0), this.lineLengths[lineIndex]);
		return this.lineStarts[lineIndex] + columnIndex;
	}

	getPosition(offset: number): Position {
		const clampedOffset = Math.min(Math.max(Math.trunc(offset), 0), this.textLengthToOffset());
		const lineIndex = Math.max(0, findLastIdxMonotonous(this.lineStarts, start => start <= clampedOffset));
		return new Position((lineIndex) + 1, (Math.min(clampedOffset - this.lineStarts[lineIndex], this.lineLengths[lineIndex])) + 1);
	}

	getLineLength(lineNumber: number): number {
		if (!Number.isSafeInteger(lineNumber) || lineNumber < 1 || lineNumber > this.lineLengths.length) throw new RangeError("Invalid line number");
		return this.lineLengths[lineNumber - 1];
	}

	private textLengthToOffset(): number { return this.lineStarts.at(-1)! + this.lineLengths.at(-1)!; }
}
