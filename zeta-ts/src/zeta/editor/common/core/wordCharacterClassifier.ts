import { Lazy } from "../../../base/common/lazy.js";
import { LRUCache } from "../../../base/common/map.js";
import { CharacterClassifier } from "./characterClassifier.js";

export enum WordCharacterClass {
	Regular = 0,
	Whitespace = 1,
	WordSeparator = 2,
}

export interface IntlWordSegmentData {
	readonly index: number;
	readonly segment: string;
	readonly isWordLike: true;
}

/** Character classes and optional locale-aware word segmentation for cursor commands. */
export class WordCharacterClassifier extends CharacterClassifier<WordCharacterClass> {
	readonly intlSegmenterLocales: readonly string[];
	private readonly segmenter: Lazy<Intl.Segmenter> | undefined;
	private cachedLine: string | undefined;
	private cachedSegments: readonly IntlWordSegmentData[] = [];

	constructor(wordSeparators: string, intlSegmenterLocales: readonly string[] = []) {
		super(WordCharacterClass.Regular);
		this.intlSegmenterLocales = Object.freeze([...intlSegmenterLocales]);
		this.segmenter = this.intlSegmenterLocales.length === 0
			? undefined
			: new Lazy(() => new Intl.Segmenter(this.intlSegmenterLocales, { granularity: "word" }));
		for (let index = 0; index < wordSeparators.length; index += 1) this.set(wordSeparators.charCodeAt(index), WordCharacterClass.WordSeparator);
		this.set(32, WordCharacterClass.Whitespace);
		this.set(9, WordCharacterClass.Whitespace);
	}

	findPrevIntlWordBeforeOrAtOffset(line: string, offset: number): IntlWordSegmentData | null {
		let candidate: IntlWordSegmentData | null = null;
		for (const segment of this.getIntlWords(line)) {
			if (segment.index > offset) break;
			candidate = segment;
		}
		return candidate;
	}

	findNextIntlWordAtOrAfterOffset(line: string, offset: number): IntlWordSegmentData | null {
		return this.getIntlWords(line).find(segment => segment.index >= offset) ?? null;
	}

	private getIntlWords(line: string): readonly IntlWordSegmentData[] {
		if (!this.segmenter) return [];
		if (this.cachedLine === line) return this.cachedSegments;
		this.cachedLine = line;
		this.cachedSegments = Object.freeze([...this.segmenter.value.segment(line)].filter(segment => segment.isWordLike).map(segment => Object.freeze({
			index: segment.index,
			segment: segment.segment,
			isWordLike: true as const,
		})));
		return this.cachedSegments;
	}
}

const classifierCache = new LRUCache<string, WordCharacterClassifier>(10);

export function getMapForWordSeparators(wordSeparators: string, intlSegmenterLocales: readonly string[] = []): WordCharacterClassifier {
	const key = `${wordSeparators}\u0000${intlSegmenterLocales.join(",")}`;
	const cached = classifierCache.get(key);
	if (cached) return cached;
	const classifier = new WordCharacterClassifier(wordSeparators, intlSegmenterLocales);
	classifierCache.set(key, classifier);
	return classifier;
}
