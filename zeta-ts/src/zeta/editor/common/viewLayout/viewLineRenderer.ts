import { Constants } from '../../../base/common/uint.js';
import { StringBuilder } from '../core/stringBuilder.js';
import { LineDecoration, LineDecorationsNormalizer } from './lineDecorations.js';
import { LinePart, LinePartMetadata } from './linePart.js';

export enum RenderWhitespace {
	None = 0,
	Boundary = 1,
	Selection = 2,
	Trailing = 3,
	All = 4,
}

export interface RenderLineInputOptions {
	readonly lineContent: string;
	readonly lineDecorations?: readonly LineDecoration[];
	readonly tabSize?: number;
	readonly renderWhitespace?: RenderWhitespace;
	readonly renderControlCharacters?: boolean;
	readonly containsRTL?: boolean;
}

/** Immutable input shared by line rendering and render-cache comparisons. */
export class RenderLineInput {
	public readonly lineContent: string;
	public readonly lineDecorations: readonly LineDecoration[];
	public readonly tabSize: number;
	public readonly renderWhitespace: RenderWhitespace;
	public readonly renderControlCharacters: boolean;
	public readonly containsRTL: boolean;

	public constructor(options: RenderLineInputOptions) {
		if (!options || typeof options.lineContent !== 'string') throw new TypeError('Render line input requires line content');
		if (!Number.isSafeInteger(options.tabSize ?? 4) || (options.tabSize ?? 4) < 1) throw new RangeError('Render line tab size must be a positive safe integer');
		this.lineContent = options.lineContent;
		this.lineDecorations = Object.freeze([...(options.lineDecorations ?? [])].sort(LineDecoration.compare));
		this.tabSize = options.tabSize ?? 4;
		this.renderWhitespace = options.renderWhitespace ?? RenderWhitespace.None;
		this.renderControlCharacters = options.renderControlCharacters ?? false;
		this.containsRTL = options.containsRTL ?? false;
	}

	public get isLTR(): boolean {
		return !this.containsRTL;
	}

	public equals(other: RenderLineInput): boolean {
		return this.lineContent === other.lineContent &&
			this.tabSize === other.tabSize &&
			this.renderWhitespace === other.renderWhitespace &&
			this.renderControlCharacters === other.renderControlCharacters &&
			this.containsRTL === other.containsRTL &&
			LineDecoration.equalsArr(this.lineDecorations, other.lineDecorations);
	}
}

export interface RenderLineOutput {
	readonly html: string;
	readonly parts: readonly LinePart[];
	readonly characterMapping: CharacterMapping;
	readonly containsForeignElements: ForeignElementType;
}

/** Renders one line without touching the DOM or owning a browser surface. */
export function renderViewLine(input: RenderLineInput): RenderLineOutput {
	const segments = LineDecorationsNormalizer.normalize(input.lineContent, input.lineDecorations);
	const parts = createLineParts(input.lineContent, segments);
	const builder = new StringBuilder(Math.max(256, input.lineContent.length * 2 + 32));
	builder.appendString('<span class="stanza-editor-line-text">');
	let startOffset = 0;
	let visibleColumn = 0;
	for (const part of parts) {
		const endOffset = part.endIndex;
		const segment = input.lineContent.slice(startOffset, endOffset);
		builder.appendString('<span');
		if (part.type.length > 0) builder.appendString(` class="${escapeAttribute(part.type)}"`);
		builder.appendString('>');
		const rendered = renderText(segment, input, visibleColumn);
		builder.appendString(rendered.html);
		builder.appendString('</span>');
		visibleColumn = rendered.endVisibleColumn;
		startOffset = endOffset;
	}
	builder.appendString('</span>');
	return Object.freeze({
		html: builder.build(),
		parts: Object.freeze(parts),
		characterMapping: createCharacterMapping(input.lineContent, parts, input.tabSize),
		containsForeignElements: parts.reduce((value, part) => value |
			(part.isPseudoBefore() ? ForeignElementType.Before : 0) |
			(part.isPseudoAfter() ? ForeignElementType.After : 0), ForeignElementType.None),
	});
}

export class DomPosition {
	constructor(
		public readonly partIndex: number,
		public readonly charIndex: number,
	) {}
}

/** Maps one source column to its rendered child span and UTF-16 offset. */
export class CharacterMapping {
	public readonly length: number;
	private readonly partIndexes: Uint32Array;
	private readonly charIndexes: Uint32Array;
	private readonly horizontalOffsets: Uint32Array;

	constructor(length: number) {
		if (!Number.isSafeInteger(length) || length < 0) throw new RangeError('Character mapping length must be a non-negative safe integer');
		this.length = length;
		this.partIndexes = new Uint32Array(length);
		this.charIndexes = new Uint32Array(length);
		this.horizontalOffsets = new Uint32Array(length);
	}

	public setColumnInfo(column: number, partIndex: number, charIndex: number, horizontalOffset: number): void {
		if (!Number.isSafeInteger(column) || column < 1 || column > this.length) throw new RangeError('Character mapping column is outside the line');
		if (!Number.isSafeInteger(partIndex) || partIndex < 0 || partIndex > Constants.MAX_UINT_32 || !Number.isSafeInteger(charIndex) || charIndex < 0 || charIndex > Constants.MAX_UINT_32) {
			throw new RangeError('Character mapping DOM position exceeds its 32-bit representation');
		}
		if (!Number.isSafeInteger(horizontalOffset) || horizontalOffset < 0) throw new RangeError('Character mapping horizontal offset must be a non-negative safe integer');
		this.partIndexes[column - 1] = partIndex;
		this.charIndexes[column - 1] = charIndex;
		this.horizontalOffsets[column - 1] = horizontalOffset;
	}

	public getHorizontalOffset(column: number): number {
		if (this.length === 0) return 0;
		return this.horizontalOffsets[this.clampColumn(column) - 1]!;
	}

	public getDomPosition(column: number): DomPosition {
		const index = this.length === 0 ? 0 : this.clampColumn(column) - 1;
		return new DomPosition(this.partIndexes[index] ?? 0, this.charIndexes[index] ?? 0);
	}

	public getColumn(domPosition: DomPosition, partLength: number): number {
		if (this.length === 0) return 1;
		let minimum = 0;
		let maximum = this.length - 1;
		while (minimum + 1 < maximum) {
			const middle = (minimum + maximum) >>> 1;
			const comparison = this.comparePosition(middle, domPosition);
			if (comparison === 0) return middle + 1;
			if (comparison > 0) maximum = middle;
			else minimum = middle;
		}
		if (this.comparePosition(minimum, domPosition) === 0) return minimum + 1;
		if (this.comparePosition(maximum, domPosition) === 0) return maximum + 1;
		const minimumPartIndex = this.partIndexes[minimum]!;
		const maximumPartIndex = this.partIndexes[maximum]!;
		const minimumCharIndex = this.charIndexes[minimum]!;
		const maximumCharIndex = minimumPartIndex === maximumPartIndex ? this.charIndexes[maximum]! : partLength;
		return domPosition.charIndex - minimumCharIndex <= maximumCharIndex - domPosition.charIndex ? minimum + 1 : maximum + 1;
	}

	private comparePosition(index: number, position: DomPosition): number {
		const partDifference = this.partIndexes[index]! - position.partIndex;
		return partDifference || this.charIndexes[index]! - position.charIndex;
	}

	private clampColumn(column: number): number {
		if (!Number.isSafeInteger(column)) throw new RangeError('Character mapping column must be a safe integer');
		return Math.min(this.length, Math.max(1, column));
	}
}

export const enum ForeignElementType {
	None = 0,
	Before = 1,
	After = 2,
}

function createLineParts(lineContent: string, segments: readonly { readonly startOffset: number; readonly endOffset: number; readonly className: string; readonly metadata: number }[]): LinePart[] {
	const boundaries = new Set<number>([0, lineContent.length]);
	for (const segment of segments) {
		boundaries.add(segment.startOffset);
		boundaries.add(segment.endOffset);
	}
	const sorted = [...boundaries].sort((left, right) => left - right);
	const parts: LinePart[] = [];
	for (let index = 0; index + 1 < sorted.length; index += 1) {
		const startOffset = sorted[index]!;
		const endOffset = sorted[index + 1]!;
		const segment = segments.find(candidate => candidate.startOffset === startOffset && candidate.endOffset === endOffset);
		const whitespaceMetadata = isWhitespace(lineContent, startOffset, endOffset) ? LinePartMetadata.IS_WHITESPACE : 0;
		parts.push(new LinePart(endOffset, segment?.className ?? '', (segment?.metadata ?? 0) | whitespaceMetadata));
	}
	if (parts.length === 0) parts.push(new LinePart(0, '', 0));
	return parts;
}

function renderText(text: string, input: RenderLineInput, startVisibleColumn: number): { readonly html: string; readonly endVisibleColumn: number } {
	let result = '';
	let visibleColumn = startVisibleColumn;
	for (const character of text) {
		if (character === '\t') {
			const spaces = input.tabSize - (visibleColumn % input.tabSize);
			result += '&nbsp;'.repeat(spaces);
			visibleColumn += spaces;
			continue;
		}
		if (character === ' ') {
			result += input.renderWhitespace === RenderWhitespace.None ? ' ' : '&nbsp;';
			visibleColumn += 1;
			continue;
		}
		result += escapeHtml(input.renderControlCharacters ? renderControlCharacter(character) : character);
		visibleColumn += 1;
	}
	return { html: result, endVisibleColumn: visibleColumn };
}

function createCharacterMapping(lineContent: string, parts: readonly LinePart[], tabSize: number): CharacterMapping {
	const mapping = new CharacterMapping(lineContent.length + 1);
	let startOffset = 0;
	let visibleColumn = 0;
	for (let partIndex = 0; partIndex < parts.length; partIndex += 1) {
		const endOffset = parts[partIndex]!.endIndex;
		let charIndex = 0;
		for (let offset = startOffset; offset < endOffset; offset += 1) {
			mapping.setColumnInfo(offset + 1, partIndex, charIndex, visibleColumn);
			if (lineContent.charCodeAt(offset) === 9) {
				const spaces = tabSize - visibleColumn % tabSize;
				charIndex += spaces;
				visibleColumn += spaces;
			} else {
				charIndex += 1;
				visibleColumn += 1;
			}
		}
		startOffset = endOffset;
		if (partIndex === parts.length - 1) mapping.setColumnInfo(lineContent.length + 1, partIndex, charIndex, visibleColumn);
	}
	return mapping;
}

function renderControlCharacter(character: string): string {
	const code = character.charCodeAt(0);
	return code < 32 ? `\\u${code.toString(16).padStart(4, '0')}` : character;
}

function isWhitespace(lineContent: string, startOffset: number, endOffset: number): boolean {
	if (startOffset === endOffset) return false;
	for (let offset = startOffset; offset < endOffset; offset += 1) {
		if (lineContent[offset] !== ' ' && lineContent[offset] !== '\t') return false;
	}
	return true;
}

function escapeHtml(value: string): string {
	return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
}

function escapeAttribute(value: string): string {
	return escapeHtml(value);
}
