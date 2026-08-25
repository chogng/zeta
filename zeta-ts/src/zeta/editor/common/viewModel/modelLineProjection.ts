import { isNonEmptyArray } from "../../../base/common/arrays.js";
import { type Event } from "../../../base/common/event.js";
import { TextPosition } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";
import { getTextGraphemeBoundaries } from "../core/textSegmentation.js";

/** Supplies logical-line visibility to a visual projection without naming a feature. */
export interface EditorLineVisibilitySource {
	readonly onDidChange: Event<void>;
	isLineVisible(lineIndex: number): boolean;
}

/** One fixed-height visual row projected from a logical TextModel line. */
export interface EditorVisualLine {
	readonly visualLineIndex: number;
	readonly logicalLineIndex: number;
	readonly startColumn: number;
	readonly endColumn: number;
	readonly firstForLogicalLine: boolean;
	readonly lastForLogicalLine: boolean;
}

/** Immutable source-to-visual-line mapping for one exact TextModel version. */
export class EditorVisualLineProjection {
	private constructor(
		readonly modelVersion: number,
		readonly logicalLineCount: number,
		private readonly projectedLines: readonly EditorVisualLine[] | undefined,
		private readonly visualLineStarts: readonly number[] | undefined,
		private readonly logicalLineVisibility: readonly boolean[] | undefined,
		private readonly identityModel: TextModel | undefined,
	) {
		Object.freeze(this);
	}

	/** Builds an unwrapped one-to-one projection without materializing one object per logical line. */
	static identity(model: TextModel): EditorVisualLineProjection {
		return new EditorVisualLineProjection(model.version, model.lineCount, undefined, undefined, undefined, model);
	}

	/** Builds a projection from one final visual segment end-column list per logical line. */
	static fromBreakColumns(model: TextModel, breakColumnsByLine: readonly (readonly number[])[]): EditorVisualLineProjection {
		if (!Array.isArray(breakColumnsByLine) || breakColumnsByLine.length !== model.lineCount) {
			throw new RangeError("Visual line break columns must contain one entry for every logical line");
		}
		const lines: EditorVisualLine[] = [];
		const visualLineStarts: number[] = [];
		for (let logicalLineIndex = 0; logicalLineIndex < model.lineCount; logicalLineIndex += 1) {
			const text = model.getLineContent(logicalLineIndex);
			const breaks = breakColumnsByLine[logicalLineIndex];
			if (!breaks) throw new RangeError("Visual line break columns must not contain holes");
			validateBreakColumns(text, breaks);
			visualLineStarts.push(lines.length);
			let startColumn = 0;
			for (let index = 0; index < breaks.length; index += 1) {
				const endColumn = breaks[index]!;
				lines.push(Object.freeze({
					visualLineIndex: lines.length,
					logicalLineIndex,
					startColumn,
					endColumn,
					firstForLogicalLine: index === 0,
					lastForLogicalLine: index + 1 === breaks.length,
				}));
				startColumn = endColumn;
			}
		}
		return new EditorVisualLineProjection(
			model.version,
			model.lineCount,
			Object.freeze(lines),
			Object.freeze(visualLineStarts),
			Object.freeze(Array.from({ length: model.lineCount }, () => true)),
			undefined,
		);
	}

	/**
	 * Builds a visual projection whose lines may omit folded logical lines.
	 *
	 * Every logical line supplies a visible visual-row anchor. A hidden line's
	 * anchor is normally its collapsed header's final row, allowing consumers to
	 * handle stale selections until their owner reveals or relocates them.
	 */
	static fromVisibleLines(modelVersion: number, logicalLineCount: number, lines: readonly EditorVisualLine[], visualLineIndexes: readonly number[]): EditorVisualLineProjection {
		if (!Number.isSafeInteger(modelVersion) || modelVersion < 0) throw new RangeError("Visual projection model version must be a non-negative safe integer");
		if (!Number.isSafeInteger(logicalLineCount) || logicalLineCount < 1) throw new RangeError("Visual projection logical line count must be a positive safe integer");
		if (!Array.isArray(visualLineIndexes) || visualLineIndexes.length !== logicalLineCount) {
			throw new RangeError("Visible visual-line anchors must contain one entry for every logical line");
		}
		const visibility = Array.from({ length: logicalLineCount }, () => false);
		const starts = Array.from({ length: logicalLineCount }, () => -1);
		const normalized = lines.map((line, visualLineIndex) => {
			if (!line || typeof line !== "object") throw new TypeError("Visible visual line must be an object");
			validateLogicalLineIndex(line.logicalLineIndex, logicalLineCount);
			if (!Number.isSafeInteger(line.startColumn) || !Number.isSafeInteger(line.endColumn) || line.startColumn < 0 || line.endColumn < line.startColumn) {
				throw new RangeError("Visible visual line columns must be ordered non-negative safe integers");
			}
			if (starts[line.logicalLineIndex] === -1) starts[line.logicalLineIndex] = visualLineIndex;
			visibility[line.logicalLineIndex] = true;
			return Object.freeze({
				visualLineIndex,
				logicalLineIndex: line.logicalLineIndex,
				startColumn: line.startColumn,
				endColumn: line.endColumn,
				firstForLogicalLine: line.firstForLogicalLine,
				lastForLogicalLine: line.lastForLogicalLine,
			});
		});
		for (let logicalLineIndex = 0; logicalLineIndex < logicalLineCount; logicalLineIndex += 1) {
			const visualLineIndex = visualLineIndexes[logicalLineIndex];
			if (!Number.isSafeInteger(visualLineIndex) || visualLineIndex < 0 || visualLineIndex >= normalized.length) {
				throw new RangeError("Visible visual-line anchor is outside the projected lines");
			}
			if (visibility[logicalLineIndex] && starts[logicalLineIndex] !== visualLineIndex) {
				throw new RangeError("Visible logical line anchor must point to its first visual row");
			}
		}
		return new EditorVisualLineProjection(
			modelVersion,
			logicalLineCount,
			Object.freeze(normalized),
			Object.freeze(visualLineIndexes.slice()),
			Object.freeze(visibility),
			undefined,
		);
	}

	get lines(): readonly EditorVisualLine[] {
		if (this.projectedLines) return this.projectedLines;
		const cached = identityLineCache.get(this);
		if (cached) return cached;
		const lines = Object.freeze(Array.from({ length: this.logicalLineCount }, (_, lineIndex) => this.identityLineAt(lineIndex)));
		identityLineCache.set(this, lines);
		return lines;
	}

	get visualLineCount(): number {
		return this.projectedLines?.length ?? this.logicalLineCount;
	}

	lineAt(visualLineIndex: number): EditorVisualLine | undefined {
		if (this.projectedLines) return this.projectedLines[visualLineIndex];
		return Number.isSafeInteger(visualLineIndex) && visualLineIndex >= 0 && visualLineIndex < this.logicalLineCount
			? this.identityLineAt(visualLineIndex)
			: undefined;
	}

	firstVisualLineIndex(logicalLineIndex: number): number {
		validateLogicalLineIndex(logicalLineIndex, this.logicalLineCount);
		return this.visualLineStarts?.[logicalLineIndex] ?? logicalLineIndex;
	}

	visualLineIndexAt(position: TextPosition): number {
		validateLogicalLineIndex(position.lineIndex, this.logicalLineCount);
		if (this.identityModel) return position.lineIndex;
		const first = this.firstVisualLineIndex(position.lineIndex);
		if (!this.logicalLineVisibility![position.lineIndex]) return first;
		const lastExclusive = position.lineIndex + 1 < this.logicalLineCount
			? this.nextVisualLineIndex(position.lineIndex + 1)
			: this.visualLineCount;
		for (let visualLineIndex = first; visualLineIndex < lastExclusive; visualLineIndex += 1) {
			const line = this.lines[visualLineIndex]!;
			if (position.columnIndex < line.endColumn || line.lastForLogicalLine) return visualLineIndex;
		}
		throw new Error("Visual line projection is inconsistent");
	}

	private nextVisualLineIndex(logicalLineIndex: number): number {
		for (let index = logicalLineIndex; index < this.logicalLineCount; index += 1) {
			if (this.logicalLineVisibility![index]) return this.visualLineStarts![index]!;
		}
		return this.visualLineCount;
	}

	private identityLineAt(lineIndex: number): EditorVisualLine {
		const model = this.identityModel!;
		if (model.version !== this.modelVersion) throw new Error("Unwrapped visual line projection is stale");
		return Object.freeze({
			visualLineIndex: lineIndex,
			logicalLineIndex: lineIndex,
			startColumn: 0,
			endColumn: model.getLineLength(lineIndex),
			firstForLogicalLine: true,
			lastForLogicalLine: true,
		});
	}
}

const identityLineCache = new WeakMap<EditorVisualLineProjection, readonly EditorVisualLine[]>();

function validateBreakColumns(text: string, breakColumns: readonly number[]): void {
	if (!isNonEmptyArray(breakColumns)) {
		throw new RangeError("Each logical line must have at least one visual segment");
	}
	if (breakColumns.length === 1 && breakColumns[0] === text.length) return;
	const boundaries = new Set(getTextGraphemeBoundaries(text));
	let previous = 0;
	for (let index = 0; index < breakColumns.length; index += 1) {
		const column = breakColumns[index];
		if (!Number.isSafeInteger(column) || column < previous || column > text.length || !boundaries.has(column)) {
			throw new RangeError("Visual line break columns must be ordered grapheme boundaries");
		}
		if (index > 0 && column === previous) {
			throw new RangeError("Only an empty logical line may contain an empty visual segment");
		}
		previous = column;
	}
	if (previous !== text.length) {
		throw new RangeError("The final visual line break must equal the logical line length");
	}
	if (text.length > 0 && breakColumns[0] === 0) {
		throw new RangeError("A non-empty logical line may not start with an empty visual segment");
	}
}

function validateLogicalLineIndex(lineIndex: number, lineCount: number): void {
	if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= lineCount) {
		throw new RangeError("Logical line index is outside the visual projection");
	}
}
