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
}

/** Renders one line without touching the DOM or owning a browser surface. */
export function renderViewLine(input: RenderLineInput): RenderLineOutput {
	const segments = LineDecorationsNormalizer.normalize(input.lineContent, input.lineDecorations);
	const parts = createLineParts(input.lineContent, segments);
	const builder = new StringBuilder(Math.max(256, input.lineContent.length * 2 + 32));
	builder.appendString('<span class="stanza-editor-line-text">');
	let startOffset = 0;
	for (const part of parts) {
		const endOffset = part.endIndex;
		const segment = input.lineContent.slice(startOffset, endOffset);
		builder.appendString('<span');
		if (part.type.length > 0) builder.appendString(` class="${escapeAttribute(part.type)}"`);
		builder.appendString('>');
		builder.appendString(renderText(segment, input, startOffset));
		builder.appendString('</span>');
		startOffset = endOffset;
	}
	builder.appendString('</span>');
	return Object.freeze({ html: builder.build(), parts: Object.freeze(parts) });
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
		parts.push(new LinePart(endOffset, segment?.className ?? '', segment?.metadata ?? 0));
	}
	if (parts.length === 0) parts.push(new LinePart(0, '', 0));
	return parts;
}

function renderText(text: string, input: RenderLineInput, offset: number): string {
	let result = '';
	let visibleColumn = offset;
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
	return result;
}

function renderControlCharacter(character: string): string {
	const code = character.charCodeAt(0);
	return code < 32 ? `\\u${code.toString(16).padStart(4, '0')}` : character;
}

function escapeHtml(value: string): string {
	return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
}

function escapeAttribute(value: string): string {
	return escapeHtml(value);
}
