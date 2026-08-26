export const enum LinePartMetadata {
	IS_WHITESPACE = 1,
	PSEUDO_BEFORE = 2,
	PSEUDO_AFTER = 4,

	IS_WHITESPACE_MASK = 0b001,
	PSEUDO_BEFORE_MASK = 0b010,
	PSEUDO_AFTER_MASK = 0b100,

	/** Compatibility aliases for callers that use the descriptive casing. */
	IsWhitespace = IS_WHITESPACE,
	PseudoBefore = PSEUDO_BEFORE,
	PseudoAfter = PSEUDO_AFTER,
	IsWhitespaceMask = IS_WHITESPACE_MASK,
	PseudoBeforeMask = PSEUDO_BEFORE_MASK,
	PseudoAfterMask = PSEUDO_AFTER_MASK,
}

/** One contiguous rendered fragment of a line. End indexes are exclusive. */
export class LinePart {
	public readonly endIndex: number;
	public readonly type: string;
	public readonly metadata: number;
	public readonly containsRTL: boolean;

	public constructor(endIndex: number, type: string, metadata = 0, containsRTL = false) {
		if (!Number.isSafeInteger(endIndex) || endIndex < 0) throw new RangeError('Line-part end index must be a non-negative safe integer');
		if (typeof type !== 'string') throw new TypeError('Line-part type must be a string');
		if (!Number.isSafeInteger(metadata) || metadata < 0) throw new RangeError('Line-part metadata must be a non-negative safe integer');
		this.endIndex = endIndex;
		this.type = type;
		this.metadata = metadata;
		this.containsRTL = containsRTL;
	}

	public isWhitespace(): boolean {
		return (this.metadata & LinePartMetadata.IS_WHITESPACE_MASK) !== 0;
	}

	public isPseudoBefore(): boolean {
		return (this.metadata & LinePartMetadata.PSEUDO_BEFORE_MASK) !== 0;
	}

	public isPseudoAfter(): boolean {
		return (this.metadata & LinePartMetadata.PSEUDO_AFTER_MASK) !== 0;
	}
}
