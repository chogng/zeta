import { CharCode } from './charCode.js';

/** Produces `a`-`z`, then `A`-`Z`, and repeats. */
export function singleLetterHash(n: number): string {
	const letterCount = CharCode.Z - CharCode.A + 1;
	n %= 2 * letterCount;
	return n < letterCount
		? String.fromCharCode(CharCode.a + n)
		: String.fromCharCode(CharCode.A + n - letterCount);
}

/** Escapes every regular-expression metacharacter in a literal string. */
export function escapeRegExpCharacters(value: string): string {
	return value.replace(/[\\^$.*+?()[\]{}|]/gu, '\\$&');
}

/** Returns the number of equal UTF-16 code units at the beginning of two strings. */
export function commonPrefixLength(left: string, right: string): number {
	const limit = Math.min(left.length, right.length);
	let index = 0;
	while (index < limit && left.charCodeAt(index) === right.charCodeAt(index)) index += 1;
	return index;
}

/** Returns the number of equal UTF-16 code units at the end of two strings. */
export function commonSuffixLength(left: string, right: string): number {
	const limit = Math.min(left.length, right.length);
	let length = 0;
	while (
		length < limit &&
		left.charCodeAt(left.length - length - 1) === right.charCodeAt(right.length - length - 1)
	) {
		length += 1;
	}
	return length;
}

export function isHighSurrogate(charCode: number): boolean {
	return charCode >= 0xD800 && charCode <= 0xDBFF;
}

export function isLowSurrogate(charCode: number): boolean {
	return charCode >= 0xDC00 && charCode <= 0xDFFF;
}

export function computeCodePoint(highSurrogate: number, lowSurrogate: number): number {
	return ((highSurrogate - 0xD800) << 10) + (lowSurrogate - 0xDC00) + 0x10000;
}

export function getNextCodePoint(value: string, length: number, offset: number): number {
	const charCode = value.charCodeAt(offset);
	if (isHighSurrogate(charCode) && offset + 1 < length) {
		const nextCharCode = value.charCodeAt(offset + 1);
		if (isLowSurrogate(nextCharCode)) return computeCodePoint(charCode, nextCharCode);
	}
	return charCode;
}

function getPrevCodePoint(value: string, offset: number): number {
	const charCode = value.charCodeAt(offset - 1);
	if (isLowSurrogate(charCode) && offset > 1) {
		const previousCharCode = value.charCodeAt(offset - 2);
		if (isHighSurrogate(previousCharCode)) return computeCodePoint(previousCharCode, charCode);
	}
	return charCode;
}

export class CodePointIterator {
	private offsetValue: number;

	constructor(private readonly value: string, offset = 0) {
		this.offsetValue = offset;
	}

	get offset(): number { return this.offsetValue; }
	setOffset(offset: number): void { this.offsetValue = offset; }
	prevCodePoint(): number {
		const codePoint = getPrevCodePoint(this.value, this.offsetValue);
		this.offsetValue -= codePoint >= 0x10000 ? 2 : 1;
		return codePoint;
	}
	nextCodePoint(): number {
		const codePoint = getNextCodePoint(this.value, this.value.length, this.offsetValue);
		this.offsetValue += codePoint >= 0x10000 ? 2 : 1;
		return codePoint;
	}
	eol(): boolean { return this.offsetValue >= this.value.length; }
}

export function isFullWidthCharacter(charCode: number): boolean {
	return (charCode >= 0x2E80 && charCode <= 0xD7AF)
		|| (charCode >= 0xF900 && charCode <= 0xFAFF)
		|| (charCode >= 0xFF01 && charCode <= 0xFF5E)
		|| (charCode >= 0xFFE0 && charCode <= 0xFFE6);
}

export function isEmojiImprecise(value: number): boolean {
	return (value >= 0x1F1E6 && value <= 0x1F1FF) || value === 8986 || value === 8987 || value === 9200
		|| value === 9203 || (value >= 9728 && value <= 10175) || value === 11088 || value === 11093
		|| (value >= 127744 && value <= 128591) || (value >= 128640 && value <= 128764)
		|| (value >= 128992 && value <= 129008) || (value >= 129280 && value <= 129535)
		|| (value >= 129648 && value <= 129782);
}

export function containsRTL(value: string): boolean {
	return /[\u0590-\u08FF\uFB1D-\uFDFD\uFE70-\uFEFC]/u.test(value);
}

export function isBasicASCII(value: string): boolean {
	return /^[\t\n\r\x20-\x7E]*$/u.test(value);
}

export function splitLines(value: string): string[] {
	return value.split(/\r\n|\r|\n/u);
}

export class GraphemeIterator {
	private readonly boundaries: readonly number[];
	private boundaryIndex: number;

	constructor(value: string, offset = 0) {
		if (!Number.isSafeInteger(offset) || offset < 0 || offset > value.length) {
			throw new RangeError('Grapheme iterator offset is outside the string');
		}
		this.boundaries = graphemeBoundaries(value);
		this.boundaryIndex = this.boundaries.indexOf(offset);
		if (this.boundaryIndex < 0) {
			throw new RangeError('Grapheme iterator offset splits a grapheme');
		}
	}

	public get offset(): number {
		return this.boundaries[this.boundaryIndex];
	}

	public nextGraphemeLength(): number {
		if (this.eol()) {
			return 0;
		}
		const start = this.offset;
		this.boundaryIndex++;
		return this.offset - start;
	}

	public prevGraphemeLength(): number {
		if (this.boundaryIndex === 0) {
			return 0;
		}
		const end = this.offset;
		this.boundaryIndex--;
		return end - this.offset;
	}

	public eol(): boolean {
		return this.boundaryIndex === this.boundaries.length - 1;
	}
}

/** Returns the UTF-16 boundaries of the grapheme containing one offset. */
export function getCharContainingOffset(value: string, offset: number): readonly [number, number] {
	if (!Number.isSafeInteger(offset) || offset < 0 || offset > value.length) {
		throw new RangeError('Grapheme offset is outside the string');
	}
	const boundaries = graphemeBoundaries(value);
	for (let boundaryIndex = 1; boundaryIndex < boundaries.length; boundaryIndex += 1) {
		const endOffset = boundaries[boundaryIndex]!;
		if (offset < endOffset) return [boundaries[boundaryIndex - 1]!, endOffset];
	}
	return [value.length, value.length];
}

export function regExpLeadsToEndlessLoop(regexp: RegExp): boolean {
	if (regexp.source === '^' || regexp.source === '^$' || regexp.source === '$' || regexp.source === '^\\s*$') return false;
	const match = regexp.exec('');
	return Boolean(match && regexp.lastIndex === 0);
}

function graphemeBoundaries(value: string): readonly number[] {
	const boundaries = [0];
	for (const segment of new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(value)) {
		const end = segment.index + segment.segment.length;
		if (end > boundaries.at(-1)!) {
			boundaries.push(end);
		}
	}
	if (boundaries.at(-1) !== value.length) {
		boundaries.push(value.length);
	}
	return Object.freeze(boundaries);
}
