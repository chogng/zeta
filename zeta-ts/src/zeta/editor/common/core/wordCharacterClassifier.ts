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
  private readonly segmenter: Intl.Segmenter | undefined;
  private cachedLine: string | undefined;
  private cachedSegments: readonly IntlWordSegmentData[] = [];

  constructor(wordSeparators: string, intlSegmenterLocales: readonly string[] = []) {
    super(WordCharacterClass.Regular);
    this.intlSegmenterLocales = Object.freeze([...intlSegmenterLocales]);
    this.segmenter = createSegmenter(this.intlSegmenterLocales);
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
    this.cachedSegments = Object.freeze([...this.segmenter.segment(line)].filter(segment => segment.isWordLike).map(segment => Object.freeze({
      index: segment.index,
      segment: segment.segment,
      isWordLike: true as const,
    })));
    return this.cachedSegments;
  }
}

const classifierCache = new Map<string, WordCharacterClassifier>();

export function getMapForWordSeparators(wordSeparators: string, intlSegmenterLocales: readonly string[] = []): WordCharacterClassifier {
  const key = `${wordSeparators}\u0000${intlSegmenterLocales.join(",")}`;
  const cached = classifierCache.get(key);
  if (cached) return cached;
  const classifier = new WordCharacterClassifier(wordSeparators, intlSegmenterLocales);
  classifierCache.set(key, classifier);
  if (classifierCache.size > 10) classifierCache.delete(classifierCache.keys().next().value!);
  return classifier;
}

function createSegmenter(locales: readonly string[]): Intl.Segmenter | undefined {
  if (locales.length === 0) return undefined;
  if (typeof Intl.Segmenter !== "function") return undefined;
  try { return new Intl.Segmenter(locales.length > 0 ? locales : undefined, { granularity: "word" }); }
  catch { return undefined; }
}
