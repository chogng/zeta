import { type LanguageWorkerDocumentSynchronization } from "../../alpha/common/languageWorkerDocumentMirror.js";
import { type LanguageToken, type LanguageTokenResult } from "../../alpha/common/languageResults.js";
import { TextPosition, TextRange, type TextSnapshot } from "../../alpha/common/text.js";
import { defaultTextMateScopeResolver, type TextMateResolvedTokenStyle, type TextMateScopeResolver } from "./textMateScopeResolver.js";
import { type TextMateGrammarContent, type TextMateGrammarRegistrySnapshot } from "./textMateGrammarRegistry.js";
import * as textMateNamespace from "vscode-textmate";
import { type IGrammar, type IOnigLib, type IRawGrammar, type Registry as TextMateRegistry, type StateStack } from "vscode-textmate";

const textMateRuntime = (textMateNamespace as unknown as { readonly default?: typeof textMateNamespace }).default ?? textMateNamespace;
const { INITIAL, Registry, parseRawGrammar } = textMateRuntime;

export interface TextMateGrammarSnapshotSource {
  readonly currentSnapshot: TextMateGrammarRegistrySnapshot;
}

export interface TextMateTokenizationCacheUpdate {
  readonly modelVersion: number;
  readonly languageId: string;
  readonly kind: "full" | "incremental";
  readonly scannedLineCount: number;
  readonly reusedLineCount: number;
}

export interface TextMateTokenizationServiceOptions {
  readonly lineTimeLimitMilliseconds?: number;
  readonly scopeResolver?: TextMateScopeResolver;
  readonly onDidUpdateCache?: (update: TextMateTokenizationCacheUpdate) => void;
}

interface TextMateRelativeToken {
  readonly startColumn: number;
  readonly endColumn: number;
  readonly tokenType: string;
  readonly modifiers: readonly string[];
}

interface TextMateLineResult {
  readonly inputState: StateStack;
  readonly outputState: StateStack;
  readonly tokens: readonly TextMateRelativeToken[];
}

interface TextMateDocumentAnalysis {
  readonly version: number;
  readonly lines: readonly string[];
  readonly lineResults: readonly TextMateLineResult[];
  readonly tokens: LanguageTokenResult;
}

interface TextMateRuntimeState {
  readonly snapshot: TextMateGrammarRegistrySnapshot;
  readonly registry: TextMateRegistry;
  readonly grammars: Map<string, Promise<IGrammar | undefined>>;
  readonly caches: Map<string, TextMateTokenizationCache>;
  users: number;
  retired: boolean;
}

/** Owns TextMate runtimes and incremental token state for one Analysis document. */
export class TextMateTokenizationService implements Disposable {
  private readonly lineTimeLimitMilliseconds: number;
  private readonly scopeResolver: TextMateScopeResolver;
  private readonly onDidUpdateCache: TextMateTokenizationServiceOptions["onDidUpdateCache"];
  private readonly onigLib: Promise<IOnigLib>;
  private currentState: TextMateRuntimeState | undefined;
  private disposed = false;

  constructor(
    private readonly grammars: TextMateGrammarSnapshotSource,
    onigLib: PromiseLike<IOnigLib>,
    options: TextMateTokenizationServiceOptions = {},
  ) {
    if (!grammars || typeof grammars !== "object" || !("currentSnapshot" in grammars)) {
      throw new TypeError("TextMate tokenization service requires a grammar snapshot source");
    }
    if (!onigLib || typeof onigLib !== "object" || typeof onigLib.then !== "function") {
      throw new TypeError("TextMate tokenization service requires an Oniguruma promise");
    }
    this.onigLib = Promise.resolve(onigLib);
    if (typeof options !== "object" || options === null) {
      throw new TypeError("TextMate tokenization service options must be an object");
    }
    this.lineTimeLimitMilliseconds = options.lineTimeLimitMilliseconds ?? 500;
    if (!Number.isSafeInteger(this.lineTimeLimitMilliseconds) || this.lineTimeLimitMilliseconds <= 0) {
      throw new RangeError("TextMate line time limit must be a positive safe integer");
    }
    this.scopeResolver = options.scopeResolver ?? defaultTextMateScopeResolver;
    if (typeof this.scopeResolver !== "function") {
      throw new TypeError("TextMate scope resolver must be a function");
    }
    this.onDidUpdateCache = options.onDidUpdateCache;
    if (this.onDidUpdateCache !== undefined && typeof this.onDidUpdateCache !== "function") {
      throw new TypeError("TextMate cache update observer must be a function");
    }
  }

  get languageIds(): readonly string[] {
    this.ensureAlive();
    return this.grammars.currentSnapshot.languageIds;
  }

  async tokenize(languageId: string, snapshot: TextSnapshot, signal: AbortSignal): Promise<LanguageTokenResult | undefined> {
    this.ensureAlive();
    signal.throwIfAborted();
    const grammarSnapshot = this.grammars.currentSnapshot;
    const definition = grammarSnapshot.getDefinitionForLanguage(languageId);
    if (!definition) return undefined;
    const state = this.acquireState(grammarSnapshot);
    try {
      const grammar = await this.getGrammar(state, definition.scopeName);
      signal.throwIfAborted();
      if (!grammar) throw new ReferenceError(`TextMate grammar '${definition.scopeName}' could not be loaded`);
      let cache = state.caches.get(languageId);
      if (!cache) {
        cache = new TextMateTokenizationCache(languageId, grammar, this.lineTimeLimitMilliseconds, this.scopeResolver, this.onDidUpdateCache);
        state.caches.set(languageId, cache);
      }
      return cache.getTokens(snapshot, signal);
    } finally {
      this.releaseState(state);
    }
  }

  synchronizeDocument(synchronization: LanguageWorkerDocumentSynchronization): void {
    this.ensureAlive();
    if (synchronization.snapshot.version !== synchronization.modelVersion) {
      throw new Error("TextMate synchronization snapshot version is inconsistent");
    }
    for (const cache of this.currentState?.caches.values() ?? []) cache.synchronizeDocument(synchronization);
  }

  /** Discards semantic styles while retaining loaded grammars after a scope-theme replacement. */
  invalidateTokenCaches(): void {
    this.ensureAlive();
    this.currentState?.caches.clear();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    if (this.currentState) {
      this.currentState.retired = true;
      this.disposeStateWhenUnused(this.currentState);
      this.currentState = undefined;
    }
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  private acquireState(snapshot: TextMateGrammarRegistrySnapshot): TextMateRuntimeState {
    let state = this.currentState;
    if (!state || state.snapshot !== snapshot) {
      if (state) {
        state.retired = true;
        this.disposeStateWhenUnused(state);
      }
      state = createRuntimeState(snapshot, this.onigLib);
      this.currentState = state;
    }
    state.users += 1;
    return state;
  }

  private releaseState(state: TextMateRuntimeState): void {
    state.users -= 1;
    this.disposeStateWhenUnused(state);
  }

  private disposeStateWhenUnused(state: TextMateRuntimeState): void {
    if (!state.retired || state.users !== 0) return;
    state.caches.clear();
    state.grammars.clear();
    state.registry.dispose();
  }

  private getGrammar(state: TextMateRuntimeState, scopeName: string): Promise<IGrammar | undefined> {
    let grammar = state.grammars.get(scopeName);
    if (!grammar) {
      grammar = state.registry.loadGrammar(scopeName).then(value => value ?? undefined);
      state.grammars.set(scopeName, grammar);
    }
    return grammar;
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("TextMateTokenizationService is already disposed");
  }
}

class TextMateTokenizationCache {
  private analysis: TextMateDocumentAnalysis | undefined;

  constructor(
    private readonly languageId: string,
    private readonly grammar: IGrammar,
    private readonly lineTimeLimitMilliseconds: number,
    private readonly scopeResolver: TextMateScopeResolver,
    private readonly onDidUpdate: TextMateTokenizationServiceOptions["onDidUpdateCache"],
  ) {}

  getTokens(snapshot: TextSnapshot, signal: AbortSignal): LanguageTokenResult {
    if (this.analysis?.version === snapshot.version) return this.analysis.tokens;
    const kind = this.analysis ? "incremental" : "full";
    this.analysis = this.update(snapshot, signal, kind);
    return this.analysis.tokens;
  }

  synchronizeDocument(synchronization: LanguageWorkerDocumentSynchronization): void {
    if (!this.analysis) return;
    if (this.analysis.version !== synchronization.previousVersion) {
      this.analysis = undefined;
      return;
    }
    this.analysis = this.update(synchronization.snapshot, undefined, "incremental");
  }

  private update(snapshot: TextSnapshot, signal: AbortSignal | undefined, kind: TextMateTokenizationCacheUpdate["kind"]): TextMateDocumentAnalysis {
    const text = snapshot.getText();
    const lines = Object.freeze(text.split("\n"));
    if (text.length !== snapshot.length || lines.length !== snapshot.lineCount) {
      throw new Error("TextMate snapshot metadata is inconsistent");
    }
    const previous = kind === "incremental" ? this.analysis : undefined;
    const scanned = previous
      ? updateLines(this.grammar, previous.lines, previous.lineResults, lines, this.lineTimeLimitMilliseconds, this.scopeResolver, signal)
      : scanAllLines(this.grammar, lines, this.lineTimeLimitMilliseconds, this.scopeResolver, signal);
    const analysis = Object.freeze({
      version: snapshot.version,
      lines,
      lineResults: scanned.lineResults,
      tokens: aggregateTokens(scanned.lineResults),
    });
    this.onDidUpdate?.(Object.freeze({
      modelVersion: snapshot.version,
      languageId: this.languageId,
      kind: previous ? "incremental" : "full",
      scannedLineCount: scanned.scannedLineCount,
      reusedLineCount: lines.length - scanned.scannedLineCount,
    }));
    return analysis;
  }
}

function createRuntimeState(snapshot: TextMateGrammarRegistrySnapshot, onigLib: Promise<IOnigLib>): TextMateRuntimeState {
  const registry = new Registry({
    onigLib,
    loadGrammar: async scopeName => {
      const definition = snapshot.getDefinitionForScope(scopeName);
      if (!definition) return undefined;
      return normalizeRawGrammar(await definition.loadGrammar(), definition.scopeName);
    },
    getInjections: scopeName => [...snapshot.getInjections(scopeName)],
  });
  return { snapshot, registry, grammars: new Map(), caches: new Map(), users: 0, retired: false };
}

function normalizeRawGrammar(content: TextMateGrammarContent, scopeName: string): IRawGrammar {
  const grammar = typeof content === "string" ? parseRawGrammar(content, `${scopeName}.tmLanguage.json`) : content;
  if (typeof grammar !== "object" || grammar === null || grammar.scopeName !== scopeName) {
    throw new TypeError(`TextMate grammar '${scopeName}' returned a different root scope`);
  }
  return grammar;
}

function scanAllLines(grammar: IGrammar, lines: readonly string[], timeLimit: number, resolver: TextMateScopeResolver, signal?: AbortSignal): { readonly lineResults: readonly TextMateLineResult[]; readonly scannedLineCount: number } {
  const lineResults: TextMateLineResult[] = [];
  let state = INITIAL;
  for (const line of lines) {
    signal?.throwIfAborted();
    const result = scanLine(grammar, line, state, timeLimit, resolver);
    lineResults.push(result);
    state = result.outputState;
  }
  return { lineResults: Object.freeze(lineResults), scannedLineCount: lines.length };
}

function updateLines(grammar: IGrammar, previousLines: readonly string[], previousResults: readonly TextMateLineResult[], lines: readonly string[], timeLimit: number, resolver: TextMateScopeResolver, signal?: AbortSignal): { readonly lineResults: readonly TextMateLineResult[]; readonly scannedLineCount: number } {
  const prefixLength = commonPrefixLength(previousLines, lines);
  const suffixLength = commonSuffixLength(previousLines, lines, prefixLength);
  const lineResults = previousResults.slice(0, prefixLength);
  const newSuffixStart = lines.length - suffixLength;
  const oldSuffixStart = previousLines.length - suffixLength;
  let state = lineResults.at(-1)?.outputState ?? INITIAL;
  let scannedLineCount = 0;
  for (let lineIndex = prefixLength; lineIndex < lines.length; lineIndex += 1) {
    signal?.throwIfAborted();
    if (lineIndex >= newSuffixStart) {
      const oldIndex = oldSuffixStart + lineIndex - newSuffixStart;
      const cached = previousResults[oldIndex]!;
      if (cached.inputState.equals(state)) {
        lineResults.push(...previousResults.slice(oldIndex));
        break;
      }
    }
    const result = scanLine(grammar, lines[lineIndex]!, state, timeLimit, resolver);
    lineResults.push(result);
    state = result.outputState;
    scannedLineCount += 1;
  }
  return { lineResults: Object.freeze(lineResults), scannedLineCount };
}

function scanLine(grammar: IGrammar, line: string, inputState: StateStack, timeLimit: number, resolver: TextMateScopeResolver): TextMateLineResult {
  const result = grammar.tokenizeLine(line, inputState, timeLimit);
  if (result.stoppedEarly) throw new Error("TextMate line tokenization exceeded its time limit");
  const tokens: TextMateRelativeToken[] = [];
  for (const token of result.tokens) {
    const startColumn = token.startIndex;
    const endColumn = Math.min(token.endIndex, line.length);
    if (!Number.isSafeInteger(startColumn) || !Number.isSafeInteger(endColumn) || startColumn < 0 || endColumn < startColumn || startColumn > line.length) {
      throw new RangeError("TextMate runtime returned an invalid token range");
    }
    if (endColumn === startColumn) continue;
    const style = resolver(Object.freeze([...token.scopes]));
    if (!style) continue;
    appendRelativeToken(tokens, startColumn, endColumn, normalizeStyle(style));
  }
  return Object.freeze({ inputState, outputState: result.ruleStack, tokens: Object.freeze(tokens) });
}

function normalizeStyle(style: TextMateResolvedTokenStyle): Required<TextMateResolvedTokenStyle> {
  if (typeof style !== "object" || style === null || typeof style.tokenType !== "string" || style.tokenType.trim() !== style.tokenType || style.tokenType.length === 0) {
    throw new TypeError("TextMate scope resolver must return a non-empty token type");
  }
  if (!Array.isArray(style.modifiers ?? [])) throw new TypeError("TextMate token modifiers must be an array");
  const modifiers = [...(style.modifiers ?? [])];
  for (const modifier of modifiers) {
    if (typeof modifier !== "string" || modifier.trim() !== modifier || modifier.length === 0) {
      throw new TypeError("TextMate token modifier must be a non-empty trimmed string");
    }
  }
  if (new Set(modifiers).size !== modifiers.length) throw new RangeError("TextMate token modifiers must be unique");
  return Object.freeze({ tokenType: style.tokenType, modifiers: Object.freeze(modifiers) });
}

function appendRelativeToken(tokens: TextMateRelativeToken[], startColumn: number, endColumn: number, style: Required<TextMateResolvedTokenStyle>): void {
  const previous = tokens.at(-1);
  if (previous && previous.endColumn === startColumn && previous.tokenType === style.tokenType && arraysEqual(previous.modifiers, style.modifiers)) {
    tokens[tokens.length - 1] = Object.freeze({ ...previous, endColumn });
    return;
  }
  tokens.push(Object.freeze({ startColumn, endColumn, tokenType: style.tokenType, modifiers: style.modifiers }));
}

function aggregateTokens(lineResults: readonly TextMateLineResult[]): LanguageTokenResult {
  const tokens: LanguageToken[] = [];
  for (let lineIndex = 0; lineIndex < lineResults.length; lineIndex += 1) {
    for (const token of lineResults[lineIndex]!.tokens) {
      tokens.push(Object.freeze({
        range: TextRange.from(TextPosition.at(lineIndex, token.startColumn), TextPosition.at(lineIndex, token.endColumn)),
        tokenType: token.tokenType,
        modifiers: token.modifiers,
      }));
    }
  }
  return Object.freeze({ tokens: Object.freeze(tokens) });
}

function commonPrefixLength(left: readonly string[], right: readonly string[]): number {
  const limit = Math.min(left.length, right.length);
  let index = 0;
  while (index < limit && left[index] === right[index]) index += 1;
  return index;
}

function commonSuffixLength(left: readonly string[], right: readonly string[], prefixLength: number): number {
  const limit = Math.min(left.length, right.length) - prefixLength;
  let length = 0;
  while (length < limit && left[left.length - length - 1] === right[right.length - length - 1]) length += 1;
  return length;
}

function arraysEqual(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}
