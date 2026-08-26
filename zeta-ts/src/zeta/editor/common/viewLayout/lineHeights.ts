import { isFiniteNumber, isPositiveSafeInteger } from '../../../base/common/numbers.js';

/** Describes one decoration-owned line-height override using one-based line numbers. */
export class CustomLineHeightData {
	public constructor(
		public readonly decorationId: string,
		public readonly startLineNumber: number,
		public readonly endLineNumber: number,
		public readonly lineHeight: number,
	) {
		if (typeof decorationId !== 'string' || decorationId.length === 0) {
			throw new TypeError('Custom line-height decoration ID must be non-empty');
		}
		if (!Number.isSafeInteger(startLineNumber) || startLineNumber < 1 ||
			!Number.isSafeInteger(endLineNumber) || endLineNumber < startLineNumber) {
			throw new RangeError('Custom line-height line numbers must be ordered positive safe integers');
		}
		if (!isFiniteNumber(lineHeight) || lineHeight < 0) {
			throw new RangeError('Custom line height must be finite and non-negative');
		}
	}
}

interface LineHeightOverride {
	readonly startLineIndex: number;
	readonly endLineIndexExclusive: number;
	readonly lineHeight: number;
}

/**
 * Owns the vertical extent of logical or visual lines.
 *
 * The default path is fixed-height and keeps the same arithmetic as the
 * viewport. Custom ranges are retained here so `LinesLayout` does not need a
 * second height policy when variable line heights are introduced.
 */
export class LineHeightsManager {
	private currentDefaultLineHeight: number;
	private readonly overrides = new Map<string, LineHeightOverride>();
	private prefixSums: readonly number[] | undefined;
	private prefixLineCount = -1;

	public constructor(defaultLineHeight: number, customLineHeightData: readonly CustomLineHeightData[] = []) {
		this.currentDefaultLineHeight = positiveLineHeight(defaultLineHeight);
		for (const data of customLineHeightData) {
			this.insertOrChangeCustomLineHeight(data.decorationId, data.startLineNumber, data.endLineNumber, data.lineHeight);
		}
	}

	public get defaultLineHeight(): number {
		return this.currentDefaultLineHeight;
	}

	public set defaultLineHeight(value: number) {
		const next = positiveLineHeight(value);
		if (next === this.currentDefaultLineHeight) return;
		this.currentDefaultLineHeight = next;
		this.invalidate();
	}

	public insertOrChangeCustomLineHeight(
		decorationId: string,
		startLineNumber: number,
		endLineNumber: number,
		lineHeight: number,
	): void {
		if (typeof decorationId !== 'string' || decorationId.length === 0) {
			throw new TypeError('Custom line-height decoration ID must be non-empty');
		}
		if (!Number.isSafeInteger(startLineNumber) || startLineNumber < 1 ||
			!Number.isSafeInteger(endLineNumber) || endLineNumber < startLineNumber) {
			throw new RangeError('Custom line-height line numbers must be ordered positive safe integers');
		}
		if (!isFiniteNumber(lineHeight) || lineHeight < 0) {
			throw new RangeError('Custom line height must be finite and non-negative');
		}
		if (lineHeight === 0) {
			this.removeCustomLineHeight(decorationId);
			return;
		}
		this.overrides.set(decorationId, Object.freeze({
			startLineIndex: startLineNumber - 1,
			endLineIndexExclusive: endLineNumber,
			lineHeight: positiveLineHeight(lineHeight),
		}));
		this.invalidate();
	}

	public removeCustomLineHeight(decorationId: string): void {
		if (this.overrides.delete(decorationId)) this.invalidate();
	}

	public heightForLineNumber(lineNumber: number): number {
		return this.heightForLineIndex(lineNumber - 1);
	}

	public heightForLineIndex(lineIndex: number): number {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0) {
			throw new RangeError('Line index must be a non-negative safe integer');
		}
		let height = this.currentDefaultLineHeight;
		for (const override of this.overrides.values()) {
			if (lineIndex >= override.startLineIndex && lineIndex < override.endLineIndexExclusive) {
				height = Math.max(height, override.lineHeight);
			}
		}
		return height;
	}

	public getAccumulatedLineHeightsIncludingLineNumber(lineNumber: number): number {
		if (!Number.isSafeInteger(lineNumber) || lineNumber < 1) {
			throw new RangeError('Line number must be a positive safe integer');
		}
		return this.getAccumulatedLineHeightsIncludingLineIndex(lineNumber - 1);
	}

	public getAccumulatedLineHeightsIncludingLineIndex(lineIndex: number): number {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0) {
			throw new RangeError('Line index must be a non-negative safe integer');
		}
		return this.getPrefixSums(lineIndex + 1)[lineIndex + 1]!;
	}

	public getVerticalOffsetForLineIndex(lineIndex: number, lineCount: number): number {
		validateLineCount(lineCount);
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex > lineCount) {
			throw new RangeError('Line index is outside the line collection');
		}
		return this.getPrefixSums(lineCount)[lineIndex]!;
	}

	public getTotalHeight(lineCount: number): number {
		validateLineCount(lineCount);
		return this.getPrefixSums(lineCount)[lineCount]!;
	}

	/** Returns the line containing the offset; an offset at a boundary selects the next line. */
	public getLineIndexAtVerticalOffset(verticalOffset: number, lineCount: number): number {
		validateLineCount(lineCount);
		if (!isFiniteNumber(verticalOffset)) throw new RangeError('Vertical offset must be finite');
		if (verticalOffset < 0) return 0;
		const prefixSums = this.getPrefixSums(lineCount);
		if (verticalOffset >= prefixSums[lineCount]!) return lineCount;
		let low = 0;
		let high = lineCount;
		while (low < high) {
			const middle = Math.floor((low + high) / 2);
			if (prefixSums[middle + 1]! <= verticalOffset) low = middle + 1;
			else high = middle;
		}
		return low;
	}

	public onLinesDeleted(fromLineNumber: number, toLineNumber: number): void {
		const fromLineIndex = positiveLineNumber(fromLineNumber) - 1;
		const toLineIndexExclusive = positiveLineNumber(toLineNumber);
		if (toLineIndexExclusive < fromLineIndex + 1) throw new RangeError('Deleted line range is invalid');
		const removedLineCount = toLineIndexExclusive - fromLineIndex;
		this.rewriteOverrides(override => {
			if (override.endLineIndexExclusive <= fromLineIndex) return override;
			if (override.startLineIndex >= toLineIndexExclusive) {
				return Object.freeze({
					...override,
					startLineIndex: override.startLineIndex - removedLineCount,
					endLineIndexExclusive: override.endLineIndexExclusive - removedLineCount,
				});
			}
			const startLineIndex = Math.min(override.startLineIndex, fromLineIndex);
			const endLineIndexExclusive = Math.max(startLineIndex + 1, override.endLineIndexExclusive - removedLineCount);
			return Object.freeze({ ...override, startLineIndex, endLineIndexExclusive });
		});
	}

	public onLinesInserted(fromLineNumber: number, toLineNumber: number): void {
		const fromLineIndex = positiveLineNumber(fromLineNumber) - 1;
		const toLineIndexExclusive = positiveLineNumber(toLineNumber);
		if (toLineIndexExclusive < fromLineIndex + 1) throw new RangeError('Inserted line range is invalid');
		const insertedLineCount = toLineIndexExclusive - fromLineIndex;
		this.rewriteOverrides(override => {
			if (override.startLineIndex >= fromLineIndex) {
				return Object.freeze({
					...override,
					startLineIndex: override.startLineIndex + insertedLineCount,
					endLineIndexExclusive: override.endLineIndexExclusive + insertedLineCount,
				});
			}
			if (override.endLineIndexExclusive > fromLineIndex) {
				return Object.freeze({
					...override,
					endLineIndexExclusive: override.endLineIndexExclusive + insertedLineCount,
				});
			}
			return override;
		});
	}

	private rewriteOverrides(transform: (override: LineHeightOverride) => LineHeightOverride): void {
		for (const [decorationId, override] of this.overrides) this.overrides.set(decorationId, transform(override));
		this.invalidate();
	}

	private getPrefixSums(lineCount: number): readonly number[] {
		if (this.prefixSums && this.prefixLineCount === lineCount) return this.prefixSums;
		const prefixSums = new Array<number>(lineCount + 1).fill(0);
		for (let lineIndex = 0; lineIndex < lineCount; lineIndex += 1) {
			prefixSums[lineIndex + 1] = prefixSums[lineIndex]! + this.heightForLineIndex(lineIndex);
		}
		this.prefixSums = Object.freeze(prefixSums);
		this.prefixLineCount = lineCount;
		return this.prefixSums;
	}

	private invalidate(): void {
		this.prefixSums = undefined;
		this.prefixLineCount = -1;
	}
}

function positiveLineHeight(value: number): number {
	if (!isFiniteNumber(value) || value <= 0) throw new RangeError('Line height must be finite and positive');
	return value;
}

function positiveLineNumber(value: number): number {
	if (!isPositiveSafeInteger(value)) throw new RangeError('Line number must be a positive safe integer');
	return value;
}

function validateLineCount(value: number): void {
	if (!isPositiveSafeInteger(value)) throw new RangeError('Line count must be a positive safe integer');
}
