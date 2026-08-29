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
