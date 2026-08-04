import { reset } from "../../../../base/browser/dom.js";
import { type Event } from "../../../../base/common/event.js";
import { type SemanticTokensModelPart } from "../../contrib/semanticTokens/common/semanticTokens.js";
import { type LanguageToken } from "../../common/tokens/languageTokens.js";
import { type TextModel } from "../../common/model/textModel.js";

export enum AlphaSemanticTokenPresentation {
  Comment = "token-comment",
  Keyword = "token-keyword",
  String = "token-string",
  Number = "token-number",
  Regexp = "token-regexp",
  Type = "token-type",
  Function = "token-function",
  Variable = "token-variable",
  Operator = "token-operator",
}

/** Fixed browser presentation modifiers recognized from LSP semantic-token data. */
export enum AlphaSemanticTokenModifier {
  Declaration = "token-modifier-declaration",
  Readonly = "token-modifier-readonly",
  Static = "token-modifier-static",
  Deprecated = "token-modifier-deprecated",
  Abstract = "token-modifier-abstract",
  Async = "token-modifier-async",
}

export interface AlphaResolvedSemanticToken {
  readonly startColumn: number;
  readonly endColumn: number;
  readonly presentation: AlphaSemanticTokenPresentation;
  /** Stable browser-only modifiers; unknown backend modifiers are excluded. */
  readonly modifiers?: readonly AlphaSemanticTokenModifier[];
}

export interface AlphaBracketColorizationSpan {
  readonly startColumn: number;
  readonly endColumn: number;
  readonly level: number;
}

export interface AlphaSemanticTokenLine {
  readonly lineIndex: number;
  readonly tokens: readonly AlphaResolvedSemanticToken[];
}

export interface AlphaSemanticTokenSource {
  readonly textModel: TextModel;
  readonly onDidChange: Event<void>;
  readonly lines: readonly AlphaSemanticTokenLine[];
  getLineTokens(lineIndex: number): readonly AlphaResolvedSemanticToken[];
}

export type AlphaSemanticTokenResolver = (token: LanguageToken) => AlphaSemanticTokenPresentation | undefined;

/**
 * Adapts one caller-owned common token index to named browser presentations.
 *
 * The source observes but owns neither the index, result store, nor text model.
 * Worker token type strings never become DOM classes directly.
 */
export function createAlphaSemanticTokenSource(
  index: SemanticTokensModelPart,
  resolvePresentation: AlphaSemanticTokenResolver = resolveAlphaSemanticTokenPresentation,
): AlphaSemanticTokenSource {
  if (typeof resolvePresentation !== "function") {
    throw new TypeError("Alpha semantic token resolver must be a function");
  }
  const onDidChange: Event<void> = listener => index.onDidChange(() => listener());
  return Object.freeze({
    textModel: index.textModel,
    onDidChange,
    get lines(): readonly AlphaSemanticTokenLine[] {
      return Object.freeze(index.lines.map(line => Object.freeze({
        lineIndex: line.lineIndex,
        tokens: resolveLineTokens(line.tokens, resolvePresentation),
      })));
    },
    getLineTokens: (lineIndex: number) => resolveLineTokens(index.getLineTokens(lineIndex), resolvePresentation),
  });
}

/** Maps common semantic-token names to Alpha's stable presentation vocabulary. */
export function resolveAlphaSemanticTokenPresentation(token: LanguageToken): AlphaSemanticTokenPresentation | undefined {
  switch (token.tokenType) {
    case "comment": return AlphaSemanticTokenPresentation.Comment;
    case "keyword":
    case "modifier": return AlphaSemanticTokenPresentation.Keyword;
    case "string": return AlphaSemanticTokenPresentation.String;
    case "number": return AlphaSemanticTokenPresentation.Number;
    case "regexp": return AlphaSemanticTokenPresentation.Regexp;
    case "class":
    case "enum":
    case "interface":
    case "namespace":
    case "struct":
    case "type":
    case "typeParameter": return AlphaSemanticTokenPresentation.Type;
    case "function":
    case "method": return AlphaSemanticTokenPresentation.Function;
    case "enumMember":
    case "event":
    case "parameter":
    case "property":
    case "variable": return AlphaSemanticTokenPresentation.Variable;
    case "operator": return AlphaSemanticTokenPresentation.Operator;
    default: return undefined;
  }
}

/** Projects one line transactionally while preserving its exact source text. */
export function projectAlphaSemanticTokenLine(
  element: HTMLElement,
  lineText: string,
  tokens: readonly AlphaResolvedSemanticToken[],
  brackets: readonly AlphaBracketColorizationSpan[] = [],
): void {
  validateLineTokens(lineText, tokens);
  validateBracketColorizations(lineText, brackets);
  if (tokens.length === 0 && brackets.length === 0) {
    element.textContent = lineText;
    return;
  }
  const ownerDocument = element.ownerDocument;
  const fragment = ownerDocument.createDocumentFragment();
  const boundaries = [...new Set([0, lineText.length, ...tokens.flatMap(token => [token.startColumn, token.endColumn]), ...brackets.flatMap(bracket => [bracket.startColumn, bracket.endColumn])])].sort((left, right) => left - right);
  for (let index = 0; index + 1 < boundaries.length; index += 1) {
    const startColumn = boundaries[index]!;
    const endColumn = boundaries[index + 1]!;
    const token = tokens.find(candidate => candidate.startColumn <= startColumn && candidate.endColumn >= endColumn);
    const bracket = brackets.find(candidate => candidate.startColumn <= startColumn && candidate.endColumn >= endColumn);
    if (!token && !bracket) {
      fragment.append(ownerDocument.createTextNode(lineText.slice(startColumn, endColumn)));
      continue;
    }
    const tokenElement = ownerDocument.createElement("span");
    tokenElement.className = "zeta-alpha-editor-token";
    if (token) tokenElement.classList.add(token.presentation);
    for (const modifier of token?.modifiers ?? []) tokenElement.classList.add(modifier);
    if (bracket) tokenElement.classList.add(`zeta-alpha-editor-bracket-level-${bracket.level}`);
    tokenElement.textContent = lineText.slice(startColumn, endColumn);
    fragment.append(tokenElement);
  }
  if (fragment.textContent !== lineText) {
    throw new Error("Alpha semantic token projection changed line text");
  }
  reset(element, fragment);
}

function validateBracketColorizations(lineText: string, brackets: readonly AlphaBracketColorizationSpan[]): void {
  let previousEnd = 0;
  for (const bracket of brackets) {
    if (!Number.isSafeInteger(bracket.startColumn) || !Number.isSafeInteger(bracket.endColumn) || bracket.startColumn < previousEnd || bracket.endColumn <= bracket.startColumn || bracket.endColumn > lineText.length) {
      throw new RangeError("Alpha bracket colorizations must be sorted, non-overlapping source ranges");
    }
    if (!Number.isSafeInteger(bracket.level) || bracket.level < 1 || bracket.level > 6) {
      throw new RangeError("Alpha bracket colorization level must be between 1 and 6");
    }
    previousEnd = bracket.endColumn;
  }
}

/** Captures and validates one source before a viewport replaces its snapshot. */
export function snapshotAlphaSemanticTokenLines(source: AlphaSemanticTokenSource): ReadonlyMap<number, readonly AlphaResolvedSemanticToken[]> {
  const result = new Map<number, readonly AlphaResolvedSemanticToken[]>();
  for (const line of source.lines) {
    if (!Number.isSafeInteger(line.lineIndex) || line.lineIndex < 0) {
      throw new RangeError("Alpha semantic token line index must be a non-negative safe integer");
    }
    if (result.has(line.lineIndex)) {
      throw new RangeError(`Duplicate Alpha semantic token line ${line.lineIndex}`);
    }
    const tokens = Object.freeze(line.tokens.map(token => Object.freeze({
      startColumn: token.startColumn,
      endColumn: token.endColumn,
      presentation: token.presentation,
      ...(token.modifiers && token.modifiers.length > 0 ? { modifiers: Object.freeze([...token.modifiers]) } : {}),
    })));
    validateLineTokens(source.textModel.getLineContent(line.lineIndex), tokens);
    result.set(line.lineIndex, tokens);
  }
  return result;
}

function validateLineTokens(lineText: string, tokens: readonly AlphaResolvedSemanticToken[]): void {
  let previousEnd = 0;
  for (const token of tokens) {
    validatePresentation(token.presentation);
    validateModifiers(token.modifiers);
    if (!Number.isSafeInteger(token.startColumn) || !Number.isSafeInteger(token.endColumn)) {
      throw new RangeError("Alpha semantic token columns must be safe integers");
    }
    if (token.startColumn < previousEnd || token.endColumn <= token.startColumn) {
      throw new RangeError("Alpha semantic tokens must be sorted, non-overlapping, and non-empty");
    }
    if (token.endColumn > lineText.length) {
      throw new RangeError("Alpha semantic token exceeds its line text");
    }
    previousEnd = token.endColumn;
  }
}

function validatePresentation(presentation: AlphaSemanticTokenPresentation): void {
  if (!Object.values(AlphaSemanticTokenPresentation).includes(presentation)) {
    throw new TypeError(`Unknown Alpha semantic token presentation '${presentation}'`);
  }
}

function validateModifiers(modifiers: readonly AlphaSemanticTokenModifier[] | undefined): void {
  if (modifiers === undefined) return;
  if (new Set(modifiers).size !== modifiers.length || modifiers.some(modifier => !Object.values(AlphaSemanticTokenModifier).includes(modifier))) {
    throw new TypeError("Unknown or duplicate Alpha semantic token modifier");
  }
}

function resolveLineTokens(tokens: readonly LanguageToken[], resolvePresentation: AlphaSemanticTokenResolver): readonly AlphaResolvedSemanticToken[] {
  const resolved: AlphaResolvedSemanticToken[] = [];
  for (const token of tokens) {
    const presentation = resolvePresentation(token);
    if (presentation === undefined) continue;
    validatePresentation(presentation);
    const modifiers = resolveAlphaSemanticTokenModifiers(token);
    resolved.push(Object.freeze({
      startColumn: token.range.start.columnIndex,
      endColumn: token.range.end.columnIndex,
      presentation,
      ...(modifiers.length > 0 ? { modifiers } : {}),
    }));
  }
  return Object.freeze(resolved);
}

/** Maps standard LSP modifier names to Alpha's closed browser presentation set. */
export function resolveAlphaSemanticTokenModifiers(token: LanguageToken): readonly AlphaSemanticTokenModifier[] {
  const resolved = new Set<AlphaSemanticTokenModifier>();
  for (const modifier of token.modifiers) {
    switch (modifier) {
      case "declaration":
      case "definition": resolved.add(AlphaSemanticTokenModifier.Declaration); break;
      case "readonly": resolved.add(AlphaSemanticTokenModifier.Readonly); break;
      case "static": resolved.add(AlphaSemanticTokenModifier.Static); break;
      case "deprecated": resolved.add(AlphaSemanticTokenModifier.Deprecated); break;
      case "abstract": resolved.add(AlphaSemanticTokenModifier.Abstract); break;
      case "async": resolved.add(AlphaSemanticTokenModifier.Async); break;
    }
  }
  return Object.freeze([...resolved]);
}
