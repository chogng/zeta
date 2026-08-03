import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type LanguageConfigurationSource, type ResolvedLanguageConfiguration } from "./languageConfiguration.js";
import { assertLanguageId } from "./languageId.js";
import { createLanguageLexicalLineScanner } from "./languageLexicalConfiguration.js";
import { type LanguageLexicalBracketEvent, type LanguageLexicalLineResult, type LanguageLexicalLineScanner, type LanguageLexicalState } from "./languageLexicalLineScanner.js";
import { type TextModelChange, type TextPosition } from "../../common/text.js";
import { type TextModel } from "../../common/textModel.js";

export interface LanguageLexicalContextSource {
  readonly textModel: TextModel;
  readonly languageId: string;
  getStructuralLineContent(lineIndex: number, startColumn?: number, endColumn?: number): string;
  getTokenTypeAt(position: TextPosition): string | undefined;
}

/** Extends lexical context with bracket events whose columns remain in source text coordinates. */
export interface LanguageStructuralBracketSource extends LanguageLexicalContextSource {
  getStructuralBracketEvents(lineIndex: number): readonly LanguageLexicalBracketEvent[];
}

/**
 * Synchronous lexical context for one model/language identity.
 *
 * The index borrows its model and configuration source. It scans lazily to the
 * requested line and invalidates the affected suffix after model changes.
 */
export class LanguageLexicalContextIndex extends DisposableOwner implements LanguageStructuralBracketSource {
  private configuration: ResolvedLanguageConfiguration | undefined;
  private scanner: LanguageLexicalLineScanner | undefined;
  private bracketTokens: readonly string[] = Object.freeze([]);
  private lineResults: LanguageLexicalLineResult[] = [];
  private disposed = false;

  constructor(readonly textModel: TextModel, readonly languageId: string, private readonly configurations: LanguageConfigurationSource) {
    super();
    assertLanguageId(languageId);
    if (!configurations || typeof configurations.getLanguageConfiguration !== "function") {
      this.dispose();
      throw new TypeError("Language lexical context requires a configuration source");
    }
    this.own(textModel.onDidChange(change => this.acceptModelChange(change)));
    this.defer(() => {
      this.disposed = true;
      this.configuration = undefined;
      this.scanner = undefined;
      this.bracketTokens = Object.freeze([]);
      this.lineResults = [];
    });
  }

  getStructuralLineContent(lineIndex: number, startColumn = 0, endColumn?: number): string {
    this.ensureAlive();
    assertLineIndex(this.textModel, lineIndex);
    const line = this.textModel.getLineContent(lineIndex);
    const resolvedEnd = endColumn ?? line.length;
    assertColumnRange(line, startColumn, resolvedEnd);
    const result = this.ensureLine(lineIndex);
    let content = "";
    let column = startColumn;
    for (const token of result.tokens) {
      if (token.endColumn <= startColumn || token.startColumn >= resolvedEnd) continue;
      const tokenStart = Math.max(startColumn, token.startColumn);
      const tokenEnd = Math.min(resolvedEnd, token.endColumn);
      if (column < tokenStart) content += line.slice(column, tokenStart);
      const tokenText = line.slice(tokenStart, tokenEnd);
      content += token.tokenType === "string" || token.tokenType === "comment" || token.tokenType === "regexp"
        ? removeTokens(tokenText, this.bracketTokens)
        : tokenText;
      column = tokenEnd;
    }
    if (column < resolvedEnd) content += line.slice(column, resolvedEnd);
    return content;
  }

  getTokenTypeAt(position: TextPosition): string | undefined {
    this.ensureAlive();
    assertLineIndex(this.textModel, position.lineIndex);
    const line = this.textModel.getLineContent(position.lineIndex);
    assertColumnRange(line, position.columnIndex, position.columnIndex);
    const result = this.ensureLine(position.lineIndex);
    const containing = result.tokens.find(token => token.startColumn <= position.columnIndex && position.columnIndex < token.endColumn);
    if (containing) return containing.tokenType;
    if (position.columnIndex === line.length && result.outputState === "blockComment") return "comment";
    if (position.columnIndex === line.length && result.outputState !== "normal" && result.outputState !== "blockComment") return "string";
    const last = result.tokens.at(-1);
    if (position.columnIndex !== line.length || last?.endColumn !== line.length) return undefined;
    const lineComment = this.configuration!.comments.lineComment;
    if (last.tokenType === "comment" && result.inputState !== "blockComment" && lineComment && line.startsWith(lineComment, last.startColumn)) {
      return "comment";
    }
    if (last.tokenType === "string" && result.events.some(event => event.kind === "diagnostic" && event.endColumn === line.length)) {
      return "string";
    }
    return undefined;
  }

  getStructuralBracketEvents(lineIndex: number): readonly LanguageLexicalBracketEvent[] {
    this.ensureAlive();
    assertLineIndex(this.textModel, lineIndex);
    return Object.freeze(this.ensureLine(lineIndex).events.flatMap(event => event.kind === "bracket" ? [event] : []));
  }

  private ensureLine(lineIndex: number): LanguageLexicalLineResult {
    const configuration = this.configurations.getLanguageConfiguration(this.languageId);
    if (configuration !== this.configuration) {
      this.configuration = configuration;
      this.scanner = createLanguageLexicalLineScanner(this.languageId, configuration);
      this.bracketTokens = structuralBracketTokens(configuration);
      this.lineResults = [];
    }
    let state: LanguageLexicalState = this.lineResults.at(-1)?.outputState ?? "normal";
    while (this.lineResults.length <= lineIndex) {
      const currentLine = this.lineResults.length;
      const result = this.scanner!.scan(this.textModel.getLineContent(currentLine), state);
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
    if (this.disposed) throw new ReferenceError("LanguageLexicalContextIndex is already disposed");
  }
}

function structuralBracketTokens(configuration: ResolvedLanguageConfiguration): readonly string[] {
  return Object.freeze([...new Set(configuration.brackets.flatMap(pair => [pair.open, pair.close]))].sort((left, right) => right.length - left.length));
}

function removeTokens(text: string, tokens: readonly string[]): string {
  let result = text;
  for (const token of tokens) result = result.split(token).join("");
  return result;
}

function assertColumnRange(line: string, startColumn: number, endColumn: number): void {
  if (!Number.isSafeInteger(startColumn) || !Number.isSafeInteger(endColumn) || startColumn < 0 || endColumn < startColumn || endColumn > line.length) {
    throw new RangeError("Language lexical context columns must describe a valid line range");
  }
}

function assertLineIndex(model: TextModel, lineIndex: number): void {
  if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= model.lineCount) {
    throw new RangeError("Language lexical context line is outside the text model");
  }
}
