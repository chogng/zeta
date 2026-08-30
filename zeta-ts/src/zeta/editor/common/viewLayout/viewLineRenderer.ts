import { StringBuilder } from '../core/stringBuilder.js';
import { OffsetRange } from '../core/ranges/offsetRange.js';
import { TextDirection } from '../model.js';
import { type IViewLineTokens } from '../tokens/lineTokens.js';
import { LineDecoration, LineDecorationsNormalizer } from './lineDecorations.js';
import { LinePart, LinePartMetadata } from './linePart.js';

export const enum RenderWhitespace {
	None = 0,
	Boundary = 1,
	Selection = 2,
	Trailing = 3,
	All = 4,
}

export interface IRenderLineInputOptions {
	useMonospaceOptimizations: boolean;
	canUseHalfwidthRightwardsArrow: boolean;
	lineContent: string;
	continuesWithWrappedLine: boolean;
	isBasicASCII: boolean;
	containsRTL: boolean;
	fauxIndentLength: number;
	lineTokens: IViewLineTokens;
	lineDecorations: LineDecoration[];
	tabSize: number;
	startVisibleColumn: number;
	spaceWidth: number;
	middotWidth: number;
	wsmiddotWidth: number;
	stopRenderingLineAfter: number;
	renderWhitespace: 'none' | 'boundary' | 'selection' | 'trailing' | 'all';
	renderControlCharacters: boolean;
	fontLigatures: boolean;
	selectionsOnLine: OffsetRange[] | null;
	textDirection: TextDirection | null;
	verticalScrollbarSize: number;
	renderNewLineWhenEmpty: boolean;
}

export class RenderLineInput {
	public readonly useMonospaceOptimizations: boolean;
	public readonly canUseHalfwidthRightwardsArrow: boolean;
	public readonly lineContent: string;
	public readonly continuesWithWrappedLine: boolean;
	public readonly isBasicASCII: boolean;
	public readonly containsRTL: boolean;
	public readonly fauxIndentLength: number;
	public readonly lineTokens: IViewLineTokens;
	public readonly lineDecorations: LineDecoration[];
	public readonly tabSize: number;
	public readonly startVisibleColumn: number;
	public readonly spaceWidth: number;
	public readonly renderSpaceWidth: number;
	public readonly renderSpaceCharCode: number;
	public readonly stopRenderingLineAfter: number;
	public readonly renderWhitespace: RenderWhitespace;
	public readonly renderControlCharacters: boolean;
	public readonly fontLigatures: boolean;
	public readonly selectionsOnLine: OffsetRange[] | null;
	public readonly textDirection: TextDirection | null;
	public readonly verticalScrollbarSize: number;
	public readonly renderNewLineWhenEmpty: boolean;

	public constructor(
		useMonospaceOptimizations: boolean,
		canUseHalfwidthRightwardsArrow: boolean,
		lineContent: string,
		continuesWithWrappedLine: boolean,
		isBasicASCII: boolean,
		containsRTL: boolean,
		fauxIndentLength: number,
		lineTokens: IViewLineTokens,
		lineDecorations: LineDecoration[],
		tabSize: number,
		startVisibleColumn: number,
		spaceWidth: number,
		middotWidth: number,
		wsmiddotWidth: number,
		stopRenderingLineAfter: number,
		renderWhitespace: 'none' | 'boundary' | 'selection' | 'trailing' | 'all',
		renderControlCharacters: boolean,
		fontLigatures: boolean,
		selectionsOnLine: OffsetRange[] | null,
		textDirection: TextDirection | null,
		verticalScrollbarSize: number,
		renderNewLineWhenEmpty: boolean = false,
	) {
		this.useMonospaceOptimizations = useMonospaceOptimizations;
		this.canUseHalfwidthRightwardsArrow = canUseHalfwidthRightwardsArrow;
		this.lineContent = lineContent;
		this.continuesWithWrappedLine = continuesWithWrappedLine;
		this.isBasicASCII = isBasicASCII;
		this.containsRTL = containsRTL;
		this.fauxIndentLength = fauxIndentLength;
		this.lineTokens = lineTokens;
		this.lineDecorations = lineDecorations.sort(LineDecoration.compare);
		this.tabSize = tabSize;
		this.startVisibleColumn = startVisibleColumn;
		this.spaceWidth = spaceWidth;
		this.stopRenderingLineAfter = stopRenderingLineAfter;
		this.renderWhitespace = renderWhitespace === 'all' ? RenderWhitespace.All
			: renderWhitespace === 'boundary' ? RenderWhitespace.Boundary
				: renderWhitespace === 'selection' ? RenderWhitespace.Selection
					: renderWhitespace === 'trailing' ? RenderWhitespace.Trailing : RenderWhitespace.None;
		this.renderControlCharacters = renderControlCharacters;
		this.fontLigatures = fontLigatures;
		this.selectionsOnLine = selectionsOnLine && selectionsOnLine.sort((left, right) => left.start < right.start ? -1 : 1);
		this.textDirection = textDirection;
		this.verticalScrollbarSize = verticalScrollbarSize;
		this.renderNewLineWhenEmpty = renderNewLineWhenEmpty;
		const useWideMiddleDot = Math.abs(wsmiddotWidth - spaceWidth) < Math.abs(middotWidth - spaceWidth);
		this.renderSpaceWidth = useWideMiddleDot ? wsmiddotWidth : middotWidth;
		this.renderSpaceCharCode = useWideMiddleDot ? 0x2E31 : 0xB7;
	}

	public get isLTR(): boolean {
		return !this.containsRTL && this.textDirection !== TextDirection.RTL;
	}

	private sameSelection(otherSelections: OffsetRange[] | null): boolean {
		if (this.selectionsOnLine === null || otherSelections === null) {
			return this.selectionsOnLine === otherSelections;
		}
		return this.selectionsOnLine.length === otherSelections.length
			&& this.selectionsOnLine.every((range, index) => range.equals(otherSelections[index]!));
	}

	public equals(other: RenderLineInput): boolean {
		return this.useMonospaceOptimizations === other.useMonospaceOptimizations &&
			this.canUseHalfwidthRightwardsArrow === other.canUseHalfwidthRightwardsArrow &&
			this.lineContent === other.lineContent &&
			this.continuesWithWrappedLine === other.continuesWithWrappedLine &&
			this.isBasicASCII === other.isBasicASCII &&
			this.containsRTL === other.containsRTL &&
			this.fauxIndentLength === other.fauxIndentLength &&
			this.tabSize === other.tabSize &&
			this.startVisibleColumn === other.startVisibleColumn &&
			this.spaceWidth === other.spaceWidth &&
			this.renderSpaceWidth === other.renderSpaceWidth &&
			this.renderSpaceCharCode === other.renderSpaceCharCode &&
			this.stopRenderingLineAfter === other.stopRenderingLineAfter &&
			this.renderWhitespace === other.renderWhitespace &&
			this.renderControlCharacters === other.renderControlCharacters &&
			this.fontLigatures === other.fontLigatures &&
			LineDecoration.equalsArr(this.lineDecorations, other.lineDecorations) &&
			this.lineTokens.equals(other.lineTokens) &&
			this.sameSelection(other.selectionsOnLine) &&
			this.textDirection === other.textDirection &&
			this.verticalScrollbarSize === other.verticalScrollbarSize &&
			this.renderNewLineWhenEmpty === other.renderNewLineWhenEmpty;
	}
}

export class RenderLineOutput {
	_renderLineOutputBrand: void = undefined;
	public readonly characterMapping: CharacterMapping;
	public readonly containsForeignElements: ForeignElementType;

	public constructor(
		characterMapping: CharacterMapping,
		containsForeignElements: ForeignElementType,
	) {
		this.characterMapping = characterMapping;
		this.containsForeignElements = containsForeignElements;
	}
}

export function renderViewLine(input: RenderLineInput, builder: StringBuilder): RenderLineOutput {
	const segments = LineDecorationsNormalizer.normalize(input.lineContent, input.lineDecorations);
	const parts = createLineParts(input.lineContent, segments);
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
	return new RenderLineOutput(
		createCharacterMapping(input.lineContent, parts, input.tabSize),
		parts.reduce((value, part) => value |
			(part.metadata & LinePartMetadata.PSEUDO_BEFORE_MASK ? ForeignElementType.Before : 0) |
			(part.isPseudoAfter() ? ForeignElementType.After : 0), ForeignElementType.None),
	);
}

export class RenderLineOutput2 {
	public constructor(
		public readonly characterMapping: CharacterMapping,
		public readonly html: string,
		public readonly containsForeignElements: ForeignElementType,
	) { }
}

export function renderViewLine2(input: RenderLineInput): RenderLineOutput2 {
	const builder = new StringBuilder(10000);
	const output = renderViewLine(input, builder);
	return new RenderLineOutput2(output.characterMapping, builder.build(), output.containsForeignElements);
}

export class DomPosition {
	constructor(
		public readonly partIndex: number,
		public readonly charIndex: number,
	) {}
}

const enum CharacterMappingConstants {
	PART_INDEX_MASK = 0b11111111111111110000000000000000,
	CHAR_INDEX_MASK = 0b00000000000000001111111111111111,
	CHAR_INDEX_OFFSET = 0,
	PART_INDEX_OFFSET = 16,
}

export class CharacterMapping {
	private static getPartIndex(partData: number): number {
		return (partData & CharacterMappingConstants.PART_INDEX_MASK) >>> CharacterMappingConstants.PART_INDEX_OFFSET;
	}

	private static getCharIndex(partData: number): number {
		return (partData & CharacterMappingConstants.CHAR_INDEX_MASK) >>> CharacterMappingConstants.CHAR_INDEX_OFFSET;
	}

	public readonly length: number;
	private readonly _data: Uint32Array;
	private readonly _horizontalOffset: Uint32Array;

	constructor(length: number, partCount: number) {
		this.length = length;
		this._data = new Uint32Array(this.length);
		this._horizontalOffset = new Uint32Array(this.length);
	}

	public setColumnInfo(column: number, partIndex: number, charIndex: number, horizontalOffset: number): void {
		const partData = ((partIndex << CharacterMappingConstants.PART_INDEX_OFFSET) | (charIndex << CharacterMappingConstants.CHAR_INDEX_OFFSET)) >>> 0;
		this._data[column - 1] = partData;
		this._horizontalOffset[column - 1] = horizontalOffset;
	}

	public getHorizontalOffset(column: number): number {
		if (this._horizontalOffset.length === 0) return 0;
		return this._horizontalOffset[column - 1]!;
	}

	private charOffsetToPartData(charOffset: number): number {
		if (this.length === 0) return 0;
		if (charOffset < 0) return this._data[0]!;
		if (charOffset >= this.length) return this._data[this.length - 1]!;
		return this._data[charOffset]!;
	}

	public getDomPosition(column: number): DomPosition {
		const partData = this.charOffsetToPartData(column - 1);
		return new DomPosition(CharacterMapping.getPartIndex(partData), CharacterMapping.getCharIndex(partData));
	}

	public getColumn(domPosition: DomPosition, partLength: number): number {
		return this.partDataToCharOffset(domPosition.partIndex, partLength, domPosition.charIndex) + 1;
	}

	private partDataToCharOffset(partIndex: number, partLength: number, charIndex: number): number {
		if (this.length === 0) return 0;
		const searchEntry = ((partIndex << CharacterMappingConstants.PART_INDEX_OFFSET) | (charIndex << CharacterMappingConstants.CHAR_INDEX_OFFSET)) >>> 0;
		let min = 0;
		let max = this.length - 1;
		while (min + 1 < max) {
			const mid = (min + max) >>> 1;
			const midEntry = this._data[mid]!;
			if (midEntry === searchEntry) return mid;
			if (midEntry > searchEntry) max = mid;
			else min = mid;
		}
		if (min === max) return min;
		const minEntry = this._data[min]!;
		const maxEntry = this._data[max]!;
		if (minEntry === searchEntry) return min;
		if (maxEntry === searchEntry) return max;
		const minPartIndex = CharacterMapping.getPartIndex(minEntry);
		const minCharIndex = CharacterMapping.getCharIndex(minEntry);
		const maxPartIndex = CharacterMapping.getPartIndex(maxEntry);
		const maxCharIndex = minPartIndex !== maxPartIndex ? partLength : CharacterMapping.getCharIndex(maxEntry);
		return charIndex - minCharIndex <= maxCharIndex - charIndex ? min : max;
	}

	public inflate() {
		const result: [number, number, number][] = [];
		for (let index = 0; index < this.length; index += 1) {
			const partData = this._data[index]!;
			result.push([
				CharacterMapping.getPartIndex(partData),
				CharacterMapping.getCharIndex(partData),
				this._horizontalOffset[index]!,
			]);
		}
		return result;
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
		parts.push(new LinePart(endOffset, segment?.className ?? '', (segment?.metadata ?? 0) | whitespaceMetadata, false));
	}
	if (parts.length === 0) parts.push(new LinePart(0, '', 0, false));
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
	const mapping = new CharacterMapping(lineContent.length + 1, parts.length);
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
