import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type LanguageConfigurationSource, type ResolvedLanguageConfiguration } from "./languageConfiguration.js";
import { assertLanguageId } from "./languageId.js";
import { createLanguageLexicalLineScanner } from "./languageLexicalConfiguration.js";
import { type LanguageLexicalBracketEvent, type LanguageLexicalLineResult, type LanguageLexicalLineScanner, type LanguageLexicalState } from "./languageLexicalLineScanner.js";
import { type TextModelChange, TextPosition, TextRange } from "../../common/text.js";
import { type TextModel } from "../../common/textModel.js";

export interface LanguageBracketMatch {
  readonly opening: TextRange;
  readonly closing: TextRange;
}

export interface LanguageBracketMatcherOptions {
  readonly maxScanLineCount?: number;
}

interface BracketLocation {
  readonly lineIndex: number;
  readonly event: LanguageLexicalBracketEvent;
}

/** Finds configured structural bracket pairs while excluding lexical string/comment spans. */
export class LanguageBracketMatcher extends DisposableOwner {
  private readonly maxScanLineCount: number;
  private configuration: ResolvedLanguageConfiguration | undefined;
  private scanner: LanguageLexicalLineScanner | undefined;
  private lineResults: LanguageLexicalLineResult[] = [];
  private disposed = false;

  constructor(
    readonly textModel: TextModel,
    readonly languageId: string,
    private readonly configurations: LanguageConfigurationSource,
    options: LanguageBracketMatcherOptions = {},
  ) {
    super();
    try {
      assertLanguageId(languageId);
      if (!configurations || typeof configurations.getLanguageConfiguration !== "function") {
        throw new TypeError("Language bracket matcher requires language configurations");
      }
      this.maxScanLineCount = readMaxScanLineCount(options.maxScanLineCount);
      this.own(textModel.onDidChange(change => this.acceptModelChange(change)));
      this.defer(() => {
        this.disposed = true;
        this.configuration = undefined;
        this.scanner = undefined;
        this.lineResults = [];
      });
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  findMatch(position: TextPosition): LanguageBracketMatch | undefined {
    this.ensureAlive();
    this.textModel.offsetAt(position);
    const candidate = this.findCandidate(position);
    if (!candidate) return undefined;
    return candidate.event.action === "open"
      ? this.findClosingMatch(candidate)
      : this.findOpeningMatch(candidate);
  }

  private findCandidate(position: TextPosition): BracketLocation | undefined {
    const events = this.bracketEventsAt(position.lineIndex);
    const contained = events.find(({ event }) => event.startColumn <= position.columnIndex && position.columnIndex < event.endColumn);
    if (contained) return contained;
    for (let index = events.length - 1; index >= 0; index -= 1) {
      const event = events[index]!;
      if (event.event.endColumn === position.columnIndex) return event;
    }
    return undefined;
  }

  private findClosingMatch(candidate: BracketLocation): LanguageBracketMatch | undefined {
    const expectedClosers = [candidate.event.matchingToken];
    const finalLine = Math.min(
      this.textModel.lineCount - 1,
      candidate.lineIndex + this.maxScanLineCount - 1,
    );
    for (let lineIndex = candidate.lineIndex; lineIndex <= finalLine; lineIndex += 1) {
      const events = this.bracketEventsAt(lineIndex);
      const startIndex = lineIndex === candidate.lineIndex
        ? events.findIndex(location => sameLocation(location, candidate)) + 1
        : 0;
      for (let index = startIndex; index < events.length; index += 1) {
        const current = events[index]!;
        if (current.event.action === "open") {
          expectedClosers.push(current.event.matchingToken);
        } else if (expectedClosers.at(-1) === current.event.token) {
          expectedClosers.pop();
          if (expectedClosers.length === 0) return match(candidate, current);
        }
      }
    }
    return undefined;
  }

  private findOpeningMatch(candidate: BracketLocation): LanguageBracketMatch | undefined {
    const expectedOpeners = [candidate.event.matchingToken];
    const firstLine = Math.max(0, candidate.lineIndex - this.maxScanLineCount + 1);
    for (let lineIndex = candidate.lineIndex; lineIndex >= firstLine; lineIndex -= 1) {
      const events = this.bracketEventsAt(lineIndex);
      const startIndex = lineIndex === candidate.lineIndex
        ? events.findIndex(location => sameLocation(location, candidate)) - 1
        : events.length - 1;
      for (let index = startIndex; index >= 0; index -= 1) {
        const current = events[index]!;
        if (current.event.action === "close") {
          expectedOpeners.push(current.event.matchingToken);
        } else if (expectedOpeners.at(-1) === current.event.token) {
          expectedOpeners.pop();
          if (expectedOpeners.length === 0) return match(current, candidate);
        }
      }
    }
    return undefined;
  }

  private bracketEventsAt(lineIndex: number): readonly BracketLocation[] {
    return Object.freeze(this.ensureLine(lineIndex).events.flatMap(event => event.kind === "bracket"
      ? [Object.freeze({ lineIndex, event })]
      : []));
  }

  private ensureLine(lineIndex: number): LanguageLexicalLineResult {
    const configuration = this.configurations.getLanguageConfiguration(this.languageId);
    if (configuration !== this.configuration) {
      this.configuration = configuration;
      this.scanner = createLanguageLexicalLineScanner(this.languageId, configuration);
      this.lineResults = [];
    }
    let state: LanguageLexicalState = this.lineResults.at(-1)?.outputState ?? "normal";
    while (this.lineResults.length <= lineIndex) {
      const currentLineIndex = this.lineResults.length;
      const result = this.scanner!.scan(this.textModel.getLineContent(currentLineIndex), state);
      this.lineResults.push(result);
      state = result.outputState;
    }
    return this.lineResults[lineIndex]!;
  }

  private acceptModelChange(change: TextModelChange): void {
    const firstChangedLine = Math.min(...change.changes.map(contentChange => contentChange.range.start.lineIndex));
    this.lineResults.length = Math.min(this.lineResults.length, firstChangedLine);
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("Language bracket matcher is already disposed");
  }
}

function match(opening: BracketLocation, closing: BracketLocation): LanguageBracketMatch {
  return Object.freeze({
    opening: TextRange.from(
      TextPosition.at(opening.lineIndex, opening.event.startColumn),
      TextPosition.at(opening.lineIndex, opening.event.endColumn),
    ),
    closing: TextRange.from(
      TextPosition.at(closing.lineIndex, closing.event.startColumn),
      TextPosition.at(closing.lineIndex, closing.event.endColumn),
    ),
  });
}

function sameLocation(left: BracketLocation, right: BracketLocation): boolean {
  return left.lineIndex === right.lineIndex &&
    left.event.startColumn === right.event.startColumn &&
    left.event.endColumn === right.event.endColumn;
}

function readMaxScanLineCount(value: number | undefined): number {
  const result = value ?? 10_000;
  if (!Number.isSafeInteger(result) || result < 1) {
    throw new RangeError("Language bracket matcher max scan line count must be a positive safe integer");
  }
  return result;
}
