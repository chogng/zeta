import { reset } from "../../../base/browser/dom.js";
import { type Event } from "../../../base/common/event.js";
import { type LanguageTokenLineIndex } from "../common/languageTokenLineIndex.js";
import { type LanguageToken } from "../common/languageResults.js";
import { type TextModel } from "../common/textModel.js";

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

export interface AlphaResolvedSemanticToken {
  readonly startColumn: number;
  readonly endColumn: number;
  readonly presentation: AlphaSemanticTokenPresentation;
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
  index: LanguageTokenLineIndex,
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
): void {
  validateLineTokens(lineText, tokens);
  if (tokens.length === 0) {
    element.textContent = lineText;
    return;
  }
  const ownerDocument = element.ownerDocument;
  const fragment = ownerDocument.createDocumentFragment();
  let column = 0;
  for (const token of tokens) {
    if (column < token.startColumn) {
      fragment.append(ownerDocument.createTextNode(lineText.slice(column, token.startColumn)));
    }
    const tokenElement = ownerDocument.createElement("span");
    tokenElement.className = "zeta-alpha-editor-token";
    tokenElement.classList.add(token.presentation);
    tokenElement.textContent = lineText.slice(token.startColumn, token.endColumn);
    fragment.append(tokenElement);
    column = token.endColumn;
  }
  if (column < lineText.length) {
    fragment.append(ownerDocument.createTextNode(lineText.slice(column)));
  }
  if (fragment.textContent !== lineText) {
    throw new Error("Alpha semantic token projection changed line text");
  }
  reset(element, fragment);
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

function resolveLineTokens(tokens: readonly LanguageToken[], resolvePresentation: AlphaSemanticTokenResolver): readonly AlphaResolvedSemanticToken[] {
  const resolved: AlphaResolvedSemanticToken[] = [];
  for (const token of tokens) {
    const presentation = resolvePresentation(token);
    if (presentation === undefined) continue;
    validatePresentation(presentation);
    resolved.push(Object.freeze({
      startColumn: token.range.start.columnIndex,
      endColumn: token.range.end.columnIndex,
      presentation,
    }));
  }
  return Object.freeze(resolved);
}
