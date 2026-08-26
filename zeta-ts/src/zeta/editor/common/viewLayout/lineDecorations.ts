import { type TextRange } from '../core/text.js';
import { LinePartMetadata } from './linePart.js';

/** Decoration kinds understood by the common line renderer. */
export enum InlineDecorationType {
	Regular = 'regular',
	RegularAffectingLetterSpacing = 'regularAffectingLetterSpacing',
	Before = 'before',
	After = 'after',
}

/** Common decoration input kept independent from browser CSS and DOM. */
export interface InlineDecoration {
	readonly range: TextRange;
	readonly inlineClassName: string;
	readonly type: InlineDecorationType;
}

/** One zero-based source span after line decorations have been clipped. */
export class LineDecoration {
	public constructor(
		public readonly startColumn: number,
		public readonly endColumn: number,
		public readonly className: string,
		public readonly type: InlineDecorationType,
	) {
		if (!Number.isSafeInteger(startColumn) || !Number.isSafeInteger(endColumn) || startColumn < 0 || endColumn < startColumn) {
			throw new RangeError('Line decoration columns must be ordered non-negative safe integers');
		}
		if (typeof className !== 'string' || className.trim().length === 0) throw new TypeError('Line decoration class name must be non-empty');
	}

	public static equalsArr(left: readonly LineDecoration[], right: readonly LineDecoration[]): boolean {
		if (left.length !== right.length) return false;
		return left.every((decoration, index) => {
			const other = right[index]!;
			return decoration.startColumn === other.startColumn && decoration.endColumn === other.endColumn &&
				decoration.className === other.className && decoration.type === other.type;
		});
	}

	public static extractWrapped(decorations: readonly LineDecoration[], startOffset: number, endOffset: number): LineDecoration[] {
		if (!Number.isSafeInteger(startOffset) || !Number.isSafeInteger(endOffset) || startOffset < 0 || endOffset < startOffset) {
			throw new RangeError('Wrapped line offsets must be ordered non-negative safe integers');
		}
		return decorations.flatMap(decoration => {
			const startColumn = Math.max(0, decoration.startColumn - startOffset);
			const endColumn = Math.min(endOffset - startOffset, decoration.endColumn - startOffset);
			return endColumn > startColumn || decoration.type === InlineDecorationType.Before || decoration.type === InlineDecorationType.After
				? [new LineDecoration(startColumn, Math.max(startColumn, endColumn), decoration.className, decoration.type)]
				: [];
		});
	}

	public static filter(
		decorations: readonly InlineDecoration[],
		lineIndex: number,
		minColumn: number,
		maxColumn: number,
	): LineDecoration[] {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || !Number.isSafeInteger(minColumn) || !Number.isSafeInteger(maxColumn) || minColumn < 0 || maxColumn < minColumn) {
			throw new RangeError('Line decoration filter coordinates are invalid');
		}
		return decorations.flatMap(decoration => {
			if (decoration.range.end.lineIndex < lineIndex || decoration.range.start.lineIndex > lineIndex) return [];
			if (decoration.range.empty && (decoration.type === InlineDecorationType.Regular || decoration.type === InlineDecorationType.RegularAffectingLetterSpacing)) return [];
			const startColumn = decoration.range.start.lineIndex === lineIndex ? decoration.range.start.columnIndex : minColumn;
			const endColumn = decoration.range.end.lineIndex === lineIndex ? decoration.range.end.columnIndex : maxColumn;
			return [new LineDecoration(Math.max(minColumn, startColumn), Math.min(maxColumn, Math.max(startColumn, endColumn)), decoration.inlineClassName, decoration.type)];
		});
	}

	public static compare(left: LineDecoration, right: LineDecoration): number {
		if (left.startColumn !== right.startColumn) return left.startColumn - right.startColumn;
		if (left.endColumn !== right.endColumn) return left.endColumn - right.endColumn;
		const typeOrder = (type: InlineDecorationType): number => type === InlineDecorationType.Before ? 0 : type === InlineDecorationType.Regular ? 1 : type === InlineDecorationType.RegularAffectingLetterSpacing ? 2 : 3;
		return typeOrder(left.type) - typeOrder(right.type) || (left.className < right.className ? -1 : left.className > right.className ? 1 : 0);
	}
}

export class DecorationSegment {
	public constructor(
		public readonly startOffset: number,
		public readonly endOffset: number,
		public readonly className: string,
		public readonly metadata: number,
	) {}
}

/** Converts overlapping line decorations into sorted, renderable segments. */
export class LineDecorationsNormalizer {
	public static normalize(lineContent: string, decorations: readonly LineDecoration[]): readonly DecorationSegment[] {
		if (typeof lineContent !== 'string') throw new TypeError('Line content must be a string');
		if (decorations.length === 0) return Object.freeze([]);
		const boundaries = new Set<number>([0, lineContent.length]);
		for (const decoration of decorations) {
			if (decoration.endColumn > lineContent.length) throw new RangeError('Line decoration exceeds line content');
			boundaries.add(decoration.startColumn);
			boundaries.add(decoration.endColumn);
		}
		const sortedBoundaries = [...boundaries].sort((left, right) => left - right);
		const result: DecorationSegment[] = [];
		for (let index = 0; index + 1 < sortedBoundaries.length; index += 1) {
			const startOffset = sortedBoundaries[index]!;
			const endOffset = sortedBoundaries[index + 1]!;
			if (endOffset <= startOffset) continue;
			const active = decorations.filter(decoration => decoration.startColumn <= startOffset && decoration.endColumn >= endOffset);
			if (active.length === 0) continue;
			result.push(new DecorationSegment(
				startOffset,
				endOffset,
				active.map(decoration => decoration.className).join(' '),
				active.reduce((metadata, decoration) => metadata | metadataFor(decoration.type), 0),
			));
		}
		for (const decoration of decorations) {
			if (decoration.startColumn !== decoration.endColumn) continue;
			const metadata = metadataFor(decoration.type);
			if (metadata === 0) continue;
			result.push(new DecorationSegment(decoration.startColumn, decoration.endColumn, decoration.className, metadata));
		}
		return Object.freeze(result.sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset));
	}
}

function metadataFor(type: InlineDecorationType): number {
	return type === InlineDecorationType.Before
		? LinePartMetadata.PSEUDO_BEFORE
		: type === InlineDecorationType.After ? LinePartMetadata.PSEUDO_AFTER : 0;
}
