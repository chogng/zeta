import { Lazy } from "../../../base/common/lazy.js";
import { LRUCache } from "../../../base/common/map.js";
import { CharacterClassifier } from "./characterClassifier.js";

export const enum WordCharacterClass {
	Regular = 0,
	Whitespace = 1,
	WordSeparator = 2,
}

/** Character classes and optional locale-aware word segmentation for cursor commands. */
export class WordCharacterClassifier extends CharacterClassifier<WordCharacterClass> {
	readonly intlSegmenterLocales: Intl.UnicodeBCP47LocaleIdentifier[];
	private readonly _segmenter: Lazy<Intl.Segmenter> | null;
	private _cachedLine: string | null = null;
	private _cachedSegments: IntlWordSegmentData[] = [];

	constructor(wordSeparators: string, intlSegmenterLocales: Intl.UnicodeBCP47LocaleIdentifier[]) {
		super(WordCharacterClass.Regular);
		this.intlSegmenterLocales = intlSegmenterLocales;
		this._segmenter = this.intlSegmenterLocales.length === 0
			? null
			: new Lazy(() => new Intl.Segmenter(this.intlSegmenterLocales, { granularity: "word" }));
		for (let index = 0; index < wordSeparators.length; index += 1) this.set(wordSeparators.charCodeAt(index), WordCharacterClass.WordSeparator);
		this.set(32, WordCharacterClass.Whitespace);
		this.set(9, WordCharacterClass.Whitespace);
	}

	findPrevIntlWordBeforeOrAtOffset(line: string, offset: number): IntlWordSegmentData | null {
		let candidate: IntlWordSegmentData | null = null;
		for (const segment of this._getIntlSegmenterWordsOnLine(line)) {
			if (segment.index > offset) break;
			candidate = segment;
		}
		return candidate;
	}

	findNextIntlWordAtOrAfterOffset(line: string, offset: number): IntlWordSegmentData | null {
		return this._getIntlSegmenterWordsOnLine(line).find(segment => segment.index >= offset) ?? null;
	}

	private _getIntlSegmenterWordsOnLine(line: string): IntlWordSegmentData[] {
		if (!this._segmenter) return [];
		if (this._cachedLine === line) return this._cachedSegments;
		this._cachedLine = line;
		this._cachedSegments = this._filterWordSegments(this._segmenter.value.segment(line));
		return this._cachedSegments;
	}

	private _filterWordSegments(segments: Intl.Segments): IntlWordSegmentData[] {
		return [...segments].filter((segment): segment is IntlWordSegmentData => this._isWordLike(segment));
	}

	private _isWordLike(segment: Intl.SegmentData): segment is IntlWordSegmentData {
		return segment.isWordLike === true;
	}
}

export interface IntlWordSegmentData extends Intl.SegmentData {
	isWordLike: true;
}

const classifierCache = new LRUCache<string, WordCharacterClassifier>(10);

export function getMapForWordSeparators(wordSeparators: string, intlSegmenterLocales: Intl.UnicodeBCP47LocaleIdentifier[]): WordCharacterClassifier {
	const key = `${wordSeparators}/${intlSegmenterLocales.join(",")}`;
	const cached = classifierCache.get(key);
	if (cached) return cached;
	const classifier = new WordCharacterClassifier(wordSeparators, intlSegmenterLocales);
	classifierCache.set(key, classifier);
	return classifier;
}
