import { type LanguageCharacterPair, type ResolvedLanguageCommentConfiguration } from "./languageConfiguration.js";
import { LanguageDiagnosticSeverity } from "./languageResults.js";

export type LanguageLexicalState = "normal" | "blockComment" | "multilineString" | `rawString:${number}`;

export interface LanguageLexicalTokenSpan {
  readonly startColumn: number;
  readonly endColumn: number;
  readonly tokenType: string;
}

export interface LanguageLexicalBracketEvent {
  readonly kind: "bracket";
  readonly action: "open" | "close";
  readonly startColumn: number;
  readonly endColumn: number;
  readonly token: string;
  readonly matchingToken: string;
}

export interface LanguageLexicalDiagnosticEvent {
  readonly kind: "diagnostic";
  readonly startColumn: number;
  readonly endColumn: number;
  readonly severity: LanguageDiagnosticSeverity;
  readonly message: string;
}

export interface LanguageLexicalMultilineEvent {
  readonly kind: "multiline";
  readonly action: "open" | "close";
  readonly lexicalKind: Exclude<LanguageLexicalState, "normal">;
  readonly columnIndex: number;
  readonly endColumn: number;
}

export type LanguageLexicalLineEvent = LanguageLexicalBracketEvent | LanguageLexicalDiagnosticEvent | LanguageLexicalMultilineEvent;

export interface LanguageLexicalLineResult {
  readonly inputState: LanguageLexicalState;
  readonly outputState: LanguageLexicalState;
  readonly tokens: readonly LanguageLexicalTokenSpan[];
  readonly events: readonly LanguageLexicalLineEvent[];
}

export interface LanguageLexicalScannerConfiguration {
  readonly comments: ResolvedLanguageCommentConfiguration;
  readonly brackets: readonly LanguageCharacterPair[];
  readonly keywords: readonly string[];
  readonly stringQuotes: readonly string[];
  readonly multilineStringQuote?: string;
  /** Prefixes which open hash-delimited raw strings, for example Rust's `r` and `br`. */
  readonly rawStringPrefixes?: readonly string[];
  /** Quote used for self-contained character literals. Invalid or unterminated input remains ordinary source text. */
  readonly characterLiteralQuote?: string;
  /** Recognizes one-line regular-expression literals for the named syntax profile. */
  readonly regularExpressionSyntax?: "ecmascript";
}

interface CompiledBracket {
  readonly action: LanguageLexicalBracketEvent["action"];
  readonly token: string;
  readonly matchingToken: string;
}

/** Immutable line scanner compiled from one language-specific lexical profile. */
export class LanguageLexicalLineScanner {
  private readonly comments: ResolvedLanguageCommentConfiguration;
  private readonly keywords: ReadonlySet<string>;
  private readonly stringQuotes: ReadonlySet<string>;
  private readonly multilineStringQuote: string | undefined;
  private readonly rawStringPrefixes: readonly string[];
  private readonly characterLiteralQuote: string | undefined;
  private readonly regularExpressionSyntax: "ecmascript" | undefined;
  private readonly brackets: readonly CompiledBracket[];

  constructor(configuration: LanguageLexicalScannerConfiguration) {
    if (typeof configuration !== "object" || configuration === null) {
      throw new TypeError("Language lexical scanner configuration must be an object");
    }
    this.comments = normalizeComments(configuration.comments);
    this.keywords = new Set(normalizeTokens(configuration.keywords, "Language lexical keyword"));
    this.stringQuotes = new Set(normalizeQuotes(configuration.stringQuotes));
    this.multilineStringQuote = configuration.multilineStringQuote === undefined
      ? undefined
      : normalizeQuote(configuration.multilineStringQuote, "Language lexical multiline string quote");
    this.rawStringPrefixes = Object.freeze([...normalizeTokens(configuration.rawStringPrefixes ?? [], "Language lexical raw string prefix")].sort((left, right) => right.length - left.length));
    this.characterLiteralQuote = configuration.characterLiteralQuote === undefined
      ? undefined
      : normalizeQuote(configuration.characterLiteralQuote, "Language lexical character literal quote");
    this.regularExpressionSyntax = configuration.regularExpressionSyntax === undefined
      ? undefined
      : normalizeRegularExpressionSyntax(configuration.regularExpressionSyntax);
    this.brackets = compileBrackets(configuration.brackets);
  }

  scan(line: string, inputState: LanguageLexicalState, signal?: AbortSignal): LanguageLexicalLineResult {
    if (typeof line !== "string") throw new TypeError("Language lexical scanner line must be a string");
    assertLexicalState(inputState);
    const tokens: LanguageLexicalTokenSpan[] = [];
    const events: LanguageLexicalLineEvent[] = [];
    let state = inputState;
    let column = 0;
    while (column < line.length) {
      signal?.throwIfAborted();
      if (state === "blockComment") {
        const close = this.comments.blockComment?.close;
        if (!close) throw new Error("Language lexical block-comment state has no closing token");
        const end = line.indexOf(close, column);
        const endColumn = end < 0 ? line.length : end + close.length;
        pushToken(tokens, column, endColumn, "comment");
        column = endColumn;
        if (end >= 0) {
          events.push(multilineEvent("close", "blockComment", end, endColumn));
          state = "normal";
        }
        continue;
      }
      if (state === "multilineString") {
        const quote = this.multilineStringQuote;
        if (!quote) throw new Error("Language lexical multiline-string state has no closing quote");
        const end = findQuote(line, column, quote);
        const endColumn = end < 0 ? line.length : end + quote.length;
        pushToken(tokens, column, endColumn, "string");
        column = endColumn;
        if (end >= 0) {
          events.push(multilineEvent("close", "multilineString", end, endColumn));
          state = "normal";
        }
        continue;
      }
      if (isRawStringState(state)) {
        const closingDelimiter = rawStringClosingDelimiter(state);
        const end = line.indexOf(closingDelimiter, column);
        const endColumn = end < 0 ? line.length : end + closingDelimiter.length;
        pushToken(tokens, column, endColumn, "string");
        column = endColumn;
        if (end >= 0) {
          events.push(multilineEvent("close", state, end, endColumn));
          state = "normal";
        }
        continue;
      }
      const character = line[column]!;
      if (isWhitespace(character)) {
        column += 1;
        continue;
      }
      const lineComment = this.comments.lineComment;
      if (lineComment && line.startsWith(lineComment, column)) {
        pushToken(tokens, column, line.length, "comment");
        break;
      }
      const blockComment = this.comments.blockComment;
      if (blockComment && line.startsWith(blockComment.open, column)) {
        const end = line.indexOf(blockComment.close, column + blockComment.open.length);
        const endColumn = end < 0 ? line.length : end + blockComment.close.length;
        pushToken(tokens, column, endColumn, "comment");
        if (end < 0) {
          events.push(multilineEvent("open", "blockComment", column, column + blockComment.open.length));
          state = "blockComment";
        }
        column = endColumn;
        continue;
      }
      if (this.regularExpressionSyntax === "ecmascript" && character === "/" && canStartEcmascriptRegularExpression(line, column)) {
        const endColumn = findEcmascriptRegularExpressionEnd(line, column);
        if (endColumn !== undefined) {
          pushToken(tokens, column, endColumn, "regexp");
          column = endColumn;
          continue;
        }
      }
      const rawString = this.findRawStringOpening(line, column);
      if (rawString) {
        const end = line.indexOf(rawString.closingDelimiter, column + rawString.openingLength);
        const endColumn = end < 0 ? line.length : end + rawString.closingDelimiter.length;
        pushToken(tokens, column, endColumn, "string");
        if (end < 0) {
          events.push(multilineEvent("open", rawString.state, column, column + rawString.openingLength));
          state = rawString.state;
        }
        column = endColumn;
        continue;
      }
      const multilineQuote = this.multilineStringQuote;
      if (multilineQuote && line.startsWith(multilineQuote, column)) {
        const end = findQuote(line, column + multilineQuote.length, multilineQuote);
        const endColumn = end < 0 ? line.length : end + multilineQuote.length;
        pushToken(tokens, column, endColumn, "string");
        if (end < 0) {
          events.push(multilineEvent("open", "multilineString", column, column + multilineQuote.length));
          state = "multilineString";
        }
        column = endColumn;
        continue;
      }
      if (this.stringQuotes.has(character)) {
        const end = findQuote(line, column + character.length, character);
        const endColumn = end < 0 ? line.length : end + character.length;
        pushToken(tokens, column, endColumn, "string");
        if (end < 0) {
          events.push(Object.freeze({
            kind: "diagnostic",
            startColumn: column,
            endColumn: line.length,
            severity: LanguageDiagnosticSeverity.Warning,
            message: "Unterminated string literal",
          }));
        }
        column = endColumn;
        continue;
      }
      if (this.characterLiteralQuote === character) {
        const endColumn = findCharacterLiteralEnd(line, column, character);
        if (endColumn !== undefined) {
          pushToken(tokens, column, endColumn, "string");
          column = endColumn;
          continue;
        }
      }
      if (isDigit(character)) {
        const end = scanNumber(line, column);
        pushToken(tokens, column, end, "number");
        column = end;
        continue;
      }
      const codePoint = readCodePoint(line, column);
      if (IDENTIFIER_START.test(codePoint.value)) {
        const end = scanIdentifier(line, column);
        const word = line.slice(column, end);
        pushToken(tokens, column, end, this.keywords.has(word) ? "keyword" : "variable");
        column = end;
        continue;
      }
      const bracket = this.brackets.find(candidate => line.startsWith(candidate.token, column));
      if (bracket) {
        events.push(Object.freeze({
          kind: "bracket",
          action: bracket.action,
          startColumn: column,
          endColumn: column + bracket.token.length,
          token: bracket.token,
          matchingToken: bracket.matchingToken,
        }));
        column += bracket.token.length;
        continue;
      }
      if (OPERATOR_CHARACTER.test(character)) {
        const end = scanOperators(line, column);
        pushToken(tokens, column, end, "operator");
        column = end;
        continue;
      }
      column += codePoint.length;
    }
    return Object.freeze({
      inputState,
      outputState: state,
      tokens: Object.freeze(tokens),
      events: Object.freeze(events),
    });
  }

  private findRawStringOpening(line: string, column: number): RawStringOpening | undefined {
    for (const prefix of this.rawStringPrefixes) {
      if (!line.startsWith(prefix, column)) continue;
      let delimiterColumn = column + prefix.length;
      while (line[delimiterColumn] === "#") delimiterColumn += 1;
      if (line[delimiterColumn] !== "\"") continue;
      const hashCount = delimiterColumn - column - prefix.length;
      const state = rawStringState(hashCount);
      return Object.freeze({
        state,
        openingLength: delimiterColumn - column + 1,
        closingDelimiter: rawStringClosingDelimiter(state),
      });
    }
    return undefined;
  }
}

interface RawStringOpening {
  readonly state: Extract<LanguageLexicalState, `rawString:${number}`>;
  readonly openingLength: number;
  readonly closingDelimiter: string;
}

const IDENTIFIER_START = /[\p{ID_Start}_$]/u;
const IDENTIFIER_CONTINUE = /[\p{ID_Continue}_$\u200c\u200d]/u;
const OPERATOR_CHARACTER = /[+\-*/%=!<>?&|^~:]/;

function normalizeComments(comments: ResolvedLanguageCommentConfiguration): ResolvedLanguageCommentConfiguration {
  if (typeof comments !== "object" || comments === null) throw new TypeError("Language lexical comments must be an object");
  return comments;
}

function normalizeTokens(tokens: readonly string[], owner: string): readonly string[] {
  if (!Array.isArray(tokens)) throw new TypeError(`${owner}s must be an array`);
  const normalized = tokens.map(token => {
    if (typeof token !== "string" || token.length === 0) throw new TypeError(`${owner} must be a non-empty string`);
    return token;
  });
  if (new Set(normalized).size !== normalized.length) throw new RangeError(`${owner}s must be unique`);
  return Object.freeze(normalized);
}

function normalizeQuotes(quotes: readonly string[]): readonly string[] {
  return normalizeTokens(quotes, "Language lexical string quote").map(quote => normalizeQuote(quote, "Language lexical string quote"));
}

function normalizeQuote(quote: string, owner: string): string {
  if ([...quote].length !== 1) throw new TypeError(`${owner} must contain one Unicode code point`);
  return quote;
}

function normalizeRegularExpressionSyntax(value: unknown): "ecmascript" {
  if (value !== "ecmascript") throw new TypeError("Unknown language lexical regular-expression syntax");
  return value;
}

function compileBrackets(pairs: readonly LanguageCharacterPair[]): readonly CompiledBracket[] {
  if (!Array.isArray(pairs)) throw new TypeError("Language lexical brackets must be an array");
  const result: CompiledBracket[] = [];
  for (const pair of pairs) {
    if (typeof pair !== "object" || pair === null || typeof pair.open !== "string" || typeof pair.close !== "string") {
      throw new TypeError("Language lexical bracket pair is invalid");
    }
    result.push(
      Object.freeze({ action: "open", token: pair.open, matchingToken: pair.close }),
      Object.freeze({ action: "close", token: pair.close, matchingToken: pair.open }),
    );
  }
  result.sort((left, right) => right.token.length - left.token.length);
  return Object.freeze(result);
}

function pushToken(tokens: LanguageLexicalTokenSpan[], startColumn: number, endColumn: number, tokenType: string): void {
  if (endColumn <= startColumn) return;
  tokens.push(Object.freeze({ startColumn, endColumn, tokenType }));
}

function multilineEvent(action: LanguageLexicalMultilineEvent["action"], lexicalKind: LanguageLexicalMultilineEvent["lexicalKind"], columnIndex: number, endColumn: number): LanguageLexicalMultilineEvent {
  return Object.freeze({ kind: "multiline", action, lexicalKind, columnIndex, endColumn });
}

function assertLexicalState(value: unknown): asserts value is LanguageLexicalState {
  if (value !== "normal" && value !== "blockComment" && value !== "multilineString" && !isRawStringState(value)) {
    throw new TypeError(`Unknown language lexical state '${String(value)}'`);
  }
}

function rawStringState(hashCount: number): Extract<LanguageLexicalState, `rawString:${number}`> {
  return `rawString:${hashCount}`;
}

function isRawStringState(value: unknown): value is Extract<LanguageLexicalState, `rawString:${number}`> {
  if (typeof value !== "string") return false;
  const match = /^rawString:(\d+)$/.exec(value);
  return match !== null && Number.isSafeInteger(Number(match[1]));
}

function rawStringClosingDelimiter(state: Extract<LanguageLexicalState, `rawString:${number}`>): string {
  const hashCount = Number(state.slice("rawString:".length));
  return `\"${"#".repeat(hashCount)}`;
}

function scanIdentifier(line: string, start: number): number {
  let column = start;
  while (column < line.length) {
    const codePoint = readCodePoint(line, column);
    if (column === start ? !IDENTIFIER_START.test(codePoint.value) : !IDENTIFIER_CONTINUE.test(codePoint.value)) break;
    column += codePoint.length;
  }
  return column;
}

function scanNumber(line: string, start: number): number {
  let column = start;
  if (line.startsWith("0x", start) || line.startsWith("0X", start)) return scanDigits(line, start + 2, /[0-9A-Fa-f_]/);
  if (line.startsWith("0b", start) || line.startsWith("0B", start)) return scanDigits(line, start + 2, /[01_]/);
  if (line.startsWith("0o", start) || line.startsWith("0O", start)) return scanDigits(line, start + 2, /[0-7_]/);
  column = scanDigits(line, column, /[0-9_]/);
  if (line[column] === ".") column = scanDigits(line, column + 1, /[0-9_]/);
  if (line[column] === "e" || line[column] === "E") {
    column += 1;
    if (line[column] === "+" || line[column] === "-") column += 1;
    column = scanDigits(line, column, /[0-9_]/);
  }
  return column;
}

function scanDigits(line: string, start: number, pattern: RegExp): number {
  let column = start;
  while (column < line.length && pattern.test(line[column]!)) column += 1;
  return column;
}

function scanOperators(line: string, start: number): number {
  let column = start + 1;
  while (column < line.length && OPERATOR_CHARACTER.test(line[column]!)) column += 1;
  return column;
}

function findQuote(line: string, start: number, quote: string): number {
  for (let column = start; column < line.length; column += 1) {
    if (line[column] === "\\") {
      column += 1;
      continue;
    }
    if (line.startsWith(quote, column)) return column;
  }
  return -1;
}

function findCharacterLiteralEnd(line: string, start: number, quote: string): number | undefined {
  let column = start + quote.length;
  if (column >= line.length) return undefined;
  if (line[column] === "\\") {
    column += 1;
    if (line[column] === "u" && line[column + 1] === "{") {
      const close = line.indexOf("}", column + 2);
      if (close < 0 || close === column + 2) return undefined;
      column = close + 1;
    } else {
      if (column >= line.length) return undefined;
      column += readCodePoint(line, column).length;
    }
  } else {
    column += readCodePoint(line, column).length;
  }
  return line.startsWith(quote, column) ? column + quote.length : undefined;
}

function canStartEcmascriptRegularExpression(line: string, column: number): boolean {
  let previous = column - 1;
  while (previous >= 0 && isWhitespace(line[previous]!)) previous -= 1;
  if (previous < 0) return true;
  if ("([{=,:;!&|?+-*%^~<>".includes(line[previous]!)) return true;
  if (!/[A-Za-z_$]/.test(line[previous]!)) return false;
  let start = previous;
  while (start > 0 && /[A-Za-z0-9_$]/.test(line[start - 1]!)) start -= 1;
  return ECMASCRIPT_REGEX_PREFIX_KEYWORDS.has(line.slice(start, previous + 1));
}

function findEcmascriptRegularExpressionEnd(line: string, start: number): number | undefined {
  let characterClass = false;
  for (let column = start + 1; column < line.length; column += 1) {
    const character = line[column]!;
    if (character === "\\") {
      column += 1;
      continue;
    }
    if (character === "[") {
      characterClass = true;
      continue;
    }
    if (character === "]") {
      characterClass = false;
      continue;
    }
    if (character !== "/" || characterClass) continue;
    let end = column + 1;
    while (end < line.length && /[A-Za-z]/.test(line[end]!)) end += 1;
    return end;
  }
  return undefined;
}

const ECMASCRIPT_REGEX_PREFIX_KEYWORDS = new Set([
  "await", "case", "delete", "do", "else", "in", "instanceof", "new", "of", "return", "throw", "typeof", "void", "yield",
]);

function readCodePoint(text: string, column: number): { readonly value: string; readonly length: number } {
  const value = String.fromCodePoint(text.codePointAt(column)!);
  return { value, length: value.length };
}

function isWhitespace(character: string): boolean {
  return /\s/u.test(character);
}

function isDigit(character: string): boolean {
  return character >= "0" && character <= "9";
}
