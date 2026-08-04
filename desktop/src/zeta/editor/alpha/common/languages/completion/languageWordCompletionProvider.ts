import { LanguageCompletionItemKind } from "./languageCompletions.js";
import { type LanguageCompletionProvider, type LanguageCompletionProviderRequest, type LanguageCompletionProviderResult } from "./languageCompletionProviders.js";
import { TextPosition, TextRange } from "../../../common/core/text.js";
import { getTextWordSegments } from "../../../common/core/textSegmentation.js";

const DEFAULT_MAXIMUM_WORD_COMPLETIONS = 100;

export interface LanguageWordCompletionProviderOptions {
  readonly id?: string;
  readonly languageIds?: readonly string[];
  readonly maximumItems?: number;
}

/** Creates a snapshot-local lexical word provider suitable for a worker realm. */
export function createLanguageWordCompletionProvider(options: LanguageWordCompletionProviderOptions = {}): LanguageCompletionProvider {
  const id = options.id ?? "language.word";
  const languageIds = Object.freeze([...(options.languageIds ?? ["*"])]);
  const maximumItems = options.maximumItems ?? DEFAULT_MAXIMUM_WORD_COMPLETIONS;
  if (!Number.isSafeInteger(maximumItems) || maximumItems <= 0) {
    throw new RangeError("Maximum word completion items must be a positive safe integer");
  }
  return Object.freeze({
    id,
    languageIds,
    provideCompletions: (request: LanguageCompletionProviderRequest, signal: AbortSignal): LanguageCompletionProviderResult | undefined => {
      signal.throwIfAborted();
      const lines = request.snapshot.getText().split("\n");
      const triggerLine = lines[request.position.lineIndex];
      if (triggerLine === undefined || request.position.columnIndex > triggerLine.length) {
        throw new RangeError("Word completion position is outside its snapshot");
      }
      const active = getTextWordSegments(triggerLine).find(segment => (
        segment.wordLike &&
        request.position.columnIndex > segment.start &&
        request.position.columnIndex <= segment.end
      ));
      if (!active) return undefined;
      const prefix = triggerLine.slice(active.start, request.position.columnIndex);
      if (prefix.length === 0) return undefined;
      const currentWord = triggerLine.slice(active.start, active.end);
      const words = new Set<string>();
      for (const line of lines) {
        signal.throwIfAborted();
        for (const segment of getTextWordSegments(line)) {
          if (!segment.wordLike) continue;
          const word = line.slice(segment.start, segment.end);
          if (word !== currentWord && word.startsWith(prefix)) words.add(word);
        }
      }
      const candidates = [...words].sort().slice(0, maximumItems);
      if (candidates.length === 0) return undefined;
      const range = TextRange.from(
        TextPosition.at(request.position.lineIndex, active.start),
        TextPosition.at(request.position.lineIndex, active.end),
      );
      return Object.freeze({
        items: Object.freeze(candidates.map(word => Object.freeze({
          id: wordIdentity(word),
          label: word,
          kind: LanguageCompletionItemKind.Text,
          range,
          insertText: word,
          sortText: word,
        }))),
        isIncomplete: words.size > maximumItems,
      });
    },
  });
}

function wordIdentity(word: string): string {
  return `word-${[...word].map(character => character.codePointAt(0)!.toString(16)).join("-")}`;
}
