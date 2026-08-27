import { reset, h, fragment as createFragment } from "../../../../base/browser/dom.js";
import { type Event } from "../../../../base/common/event.js";
import { combinedDisposable, type IDisposable } from "../../../../base/common/lifecycle.js";
import { type LanguageToken } from "../../../common/tokens/languageTokens.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { CharacterMapping } from '../../../common/viewLayout/viewLineRenderer.js';

export enum SemanticTokenPresentation {
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
export enum SemanticTokenModifier {
	Declaration = "token-modifier-declaration",
	Readonly = "token-modifier-readonly",
	Static = "token-modifier-static",
	Deprecated = "token-modifier-deprecated",
	Abstract = "token-modifier-abstract",
	Async = "token-modifier-async",
}

export interface ResolvedSemanticToken {
	readonly startColumn: number;
	readonly endColumn: number;
	readonly presentation?: SemanticTokenPresentation;
	/** Stable browser-only modifiers; unknown backend modifiers are excluded. */
	readonly modifiers?: readonly SemanticTokenModifier[];
	readonly syntaxPresentation?: LanguageToken["presentation"];
}

export interface BracketColorizationSpan {
	readonly startColumn: number;
	readonly endColumn: number;
	readonly level: number;
}

export interface SemanticTokenLine {
	readonly lineIndex: number;
	readonly tokens: readonly ResolvedSemanticToken[];
}

export interface SemanticTokenSource {
	readonly textModel: TextModel;
	readonly onDidChange: Event<void>;
	readonly lines: readonly SemanticTokenLine[];
	getLineTokens(lineIndex: number): readonly ResolvedSemanticToken[];
}

/** Feature-neutral bracket projection consumed by the browser viewport. */
export interface BracketColorizationSource {
	readonly textModel: TextModel;
	getLineBrackets(lineIndex: number): readonly BracketColorizationSpan[];
}

/** Minimal token model contract adapted by the browser projection. */
export interface SemanticTokenModelSource {
	readonly textModel: TextModel;
	readonly onDidChange: (listener: (...args: any[]) => void) => IDisposable;
	readonly lines: readonly { readonly lineIndex: number; readonly tokens: readonly LanguageToken[] }[];
	getLineTokens(lineIndex: number): readonly LanguageToken[];
}

export type SemanticTokenResolver = (token: LanguageToken) => SemanticTokenPresentation | undefined;

/**
 * Adapts one caller-owned common token index to named browser presentations.
 *
 * The source observes but owns neither the index, result store, nor text model.
 * Worker token type strings never become DOM classes directly.
 */
export function createStanzaSemanticTokenSource(
	index: SemanticTokenModelSource,
	resolvePresentation: SemanticTokenResolver = resolveStanzaSemanticTokenPresentation,
): SemanticTokenSource {
	if (typeof resolvePresentation !== "function") {
		throw new TypeError("Stanza semantic token resolver must be a function");
	}
	const onDidChange: Event<void> = listener => index.onDidChange(() => listener());
	return Object.freeze({
		textModel: index.textModel,
		onDidChange,
		get lines(): readonly SemanticTokenLine[] {
			return Object.freeze(index.lines.map(line => Object.freeze({
				lineIndex: line.lineIndex,
				tokens: resolveLineTokens(line.tokens, resolvePresentation),
			})));
		},
		getLineTokens: (lineIndex: number) => resolveLineTokens(index.getLineTokens(lineIndex), resolvePresentation),
	});
}

/** Overlays server semantic classification on lexical tokens without losing uncovered syntax styling. */
export function createOverlaySemanticTokenSource(base: SemanticTokenSource, overlay: SemanticTokenSource): SemanticTokenSource {
	if (base.textModel !== overlay.textModel) throw new TypeError("Semantic-token overlay sources must share one text model");
	const onDidChange: Event<void> = listener => combinedDisposable(base.onDidChange(listener), overlay.onDidChange(listener));
	const getLineTokens = (lineIndex: number): readonly ResolvedSemanticToken[] => mergeResolvedLineTokens(base.getLineTokens(lineIndex), overlay.getLineTokens(lineIndex));
	return Object.freeze({
		textModel: base.textModel,
		onDidChange,
		get lines(): readonly SemanticTokenLine[] {
			const lineIndexes = new Set([...base.lines.map(line => line.lineIndex), ...overlay.lines.map(line => line.lineIndex)]);
			return Object.freeze([...lineIndexes].sort((left, right) => left - right).map(lineIndex => Object.freeze({ lineIndex, tokens: getLineTokens(lineIndex) })));
		},
		getLineTokens,
	});
}

/** Maps common semantic-token names to Stanza's stable presentation vocabulary. */
export function resolveStanzaSemanticTokenPresentation(token: LanguageToken): SemanticTokenPresentation | undefined {
	switch (token.tokenType) {
		case "comment": return SemanticTokenPresentation.Comment;
		case "keyword":
		case "modifier": return SemanticTokenPresentation.Keyword;
		case "string": return SemanticTokenPresentation.String;
		case "number": return SemanticTokenPresentation.Number;
		case "regexp": return SemanticTokenPresentation.Regexp;
		case "class":
		case "enum":
		case "interface":
		case "namespace":
		case "struct":
		case "type":
		case "typeParameter": return SemanticTokenPresentation.Type;
		case "function":
		case "method": return SemanticTokenPresentation.Function;
		case "enumMember":
		case "event":
		case "parameter":
		case "property":
		case "variable": return SemanticTokenPresentation.Variable;
		case "operator": return SemanticTokenPresentation.Operator;
		default: return undefined;
	}
}

/** Projects one line transactionally while preserving its exact source text. */
export function projectStanzaSemanticTokenLine(
	element: HTMLElement,
	lineText: string,
	tokens: readonly ResolvedSemanticToken[],
	brackets: readonly BracketColorizationSpan[] = [],
	tabSize = 4,
): CharacterMapping {
	validateLineTokens(lineText, tokens);
	validateBracketColorizations(lineText, brackets);
	if (!Number.isSafeInteger(tabSize) || tabSize < 1) throw new RangeError('Stanza semantic line tab size must be a positive safe integer');
	const ownerDocument = element.ownerDocument;
	const fragment = createFragment(ownerDocument);
	const characterMapping = new CharacterMapping(lineText.length + 1);
	const boundaries = [...new Set([0, lineText.length, ...tokens.flatMap(token => [token.startColumn, token.endColumn]), ...brackets.flatMap(bracket => [bracket.startColumn, bracket.endColumn])])].sort((left, right) => left - right);
	let visibleColumn = 0;
	if (lineText.length === 0) {
		fragment.append(h(ownerDocument, 'span'));
		characterMapping.setColumnInfo(1, 0, 0, 0);
	}
	for (let index = 0; index + 1 < boundaries.length; index += 1) {
		const startColumn = boundaries[index]!;
		const endColumn = boundaries[index + 1]!;
		const token = tokens.find(candidate => candidate.startColumn <= startColumn && candidate.endColumn >= endColumn);
		const bracket = brackets.find(candidate => candidate.startColumn <= startColumn && candidate.endColumn >= endColumn);
		const tokenElement = h(ownerDocument, "span");
		if (token || bracket) tokenElement.className = "stanza-editor-token";
		if (token?.presentation) tokenElement.classList.add(token.presentation);
		for (const modifier of token?.modifiers ?? []) tokenElement.classList.add(modifier);
		if (token?.syntaxPresentation) applySyntaxPresentation(tokenElement, token.syntaxPresentation);
		if (bracket) tokenElement.classList.add(`stanza-editor-bracket-level-${bracket.level}`);
		tokenElement.textContent = lineText.slice(startColumn, endColumn);
		for (let offset = startColumn; offset < endColumn; offset += 1) {
			characterMapping.setColumnInfo(offset + 1, index, offset - startColumn, visibleColumn);
			visibleColumn += lineText.charCodeAt(offset) === 9 ? tabSize - visibleColumn % tabSize : 1;
		}
		if (endColumn === lineText.length) characterMapping.setColumnInfo(lineText.length + 1, index, endColumn - startColumn, visibleColumn);
		fragment.append(tokenElement);
	}
	if (fragment.textContent !== lineText) {
		throw new Error("Stanza semantic token projection changed line text");
	}
	reset(element, fragment);
	return characterMapping;
}

function validateBracketColorizations(lineText: string, brackets: readonly BracketColorizationSpan[]): void {
	let previousEnd = 0;
	for (const bracket of brackets) {
		if (!Number.isSafeInteger(bracket.startColumn) || !Number.isSafeInteger(bracket.endColumn) || bracket.startColumn < previousEnd || bracket.endColumn <= bracket.startColumn || bracket.endColumn > lineText.length) {
			throw new RangeError("Stanza bracket colorizations must be sorted, non-overlapping source ranges");
		}
		if (!Number.isSafeInteger(bracket.level) || bracket.level < 1 || bracket.level > 6) {
			throw new RangeError("Stanza bracket colorization level must be between 1 and 6");
		}
		previousEnd = bracket.endColumn;
	}
}

/** Captures and validates one source before a viewport replaces its snapshot. */
export function snapshotStanzaSemanticTokenLines(source: SemanticTokenSource): ReadonlyMap<number, readonly ResolvedSemanticToken[]> {
	const result = new Map<number, readonly ResolvedSemanticToken[]>();
	for (const line of source.lines) {
		if (!Number.isSafeInteger(line.lineIndex) || line.lineIndex < 0) {
			throw new RangeError("Stanza semantic token line index must be a non-negative safe integer");
		}
		if (result.has(line.lineIndex)) {
			throw new RangeError(`Duplicate Stanza semantic token line ${line.lineIndex}`);
		}
		const tokens = Object.freeze(line.tokens.map(token => Object.freeze({
			startColumn: token.startColumn,
			endColumn: token.endColumn,
			presentation: token.presentation,
			...(token.modifiers && token.modifiers.length > 0 ? { modifiers: Object.freeze([...token.modifiers]) } : {}),
			...(token.syntaxPresentation === undefined ? {} : { syntaxPresentation: token.syntaxPresentation }),
		})));
		validateLineTokens(source.textModel.getLineContent(line.lineIndex), tokens);
		result.set(line.lineIndex, tokens);
	}
	return result;
}

function validateLineTokens(lineText: string, tokens: readonly ResolvedSemanticToken[]): void {
	let previousEnd = 0;
	for (const token of tokens) {
		if (token.presentation !== undefined) validatePresentation(token.presentation);
		validateModifiers(token.modifiers);
		if (!Number.isSafeInteger(token.startColumn) || !Number.isSafeInteger(token.endColumn)) {
			throw new RangeError("Stanza semantic token columns must be safe integers");
		}
		if (token.startColumn < previousEnd || token.endColumn <= token.startColumn) {
			throw new RangeError("Stanza semantic tokens must be sorted, non-overlapping, and non-empty");
		}
		if (token.endColumn > lineText.length) {
			throw new RangeError("Stanza semantic token exceeds its line text");
		}
		previousEnd = token.endColumn;
	}
}

function validatePresentation(presentation: SemanticTokenPresentation): void {
	if (!Object.values(SemanticTokenPresentation).includes(presentation)) {
		throw new TypeError(`Unknown Stanza semantic token presentation '${presentation}'`);
	}
}

function validateModifiers(modifiers: readonly SemanticTokenModifier[] | undefined): void {
	if (modifiers === undefined) return;
	if (new Set(modifiers).size !== modifiers.length || modifiers.some(modifier => !Object.values(SemanticTokenModifier).includes(modifier))) {
		throw new TypeError("Unknown or duplicate Stanza semantic token modifier");
	}
}

function resolveLineTokens(tokens: readonly LanguageToken[], resolvePresentation: SemanticTokenResolver): readonly ResolvedSemanticToken[] {
	const resolved: ResolvedSemanticToken[] = [];
	for (const token of tokens) {
		const presentation = resolvePresentation(token);
		if (presentation === undefined && token.presentation === undefined) continue;
		if (presentation !== undefined) validatePresentation(presentation);
		const modifiers = resolveStanzaSemanticTokenModifiers(token);
		resolved.push(Object.freeze({
			startColumn: token.range.start.columnIndex,
			endColumn: token.range.end.columnIndex,
			...(presentation === undefined ? {} : { presentation }),
			...(modifiers.length > 0 ? { modifiers } : {}),
			...(token.presentation === undefined ? {} : { syntaxPresentation: token.presentation }),
		}));
	}
	return Object.freeze(resolved);
}

function mergeResolvedLineTokens(base: readonly ResolvedSemanticToken[], overlay: readonly ResolvedSemanticToken[]): readonly ResolvedSemanticToken[] {
	if (overlay.length === 0) return base;
	if (base.length === 0) return overlay;
	const boundaries = [...new Set([...base.flatMap(token => [token.startColumn, token.endColumn]), ...overlay.flatMap(token => [token.startColumn, token.endColumn])])].sort((left, right) => left - right);
	const result: ResolvedSemanticToken[] = [];
	for (let index = 0; index + 1 < boundaries.length; index += 1) {
		const startColumn = boundaries[index]!;
		const endColumn = boundaries[index + 1]!;
		const semantic = overlay.find(token => token.startColumn <= startColumn && token.endColumn >= endColumn);
		const lexical = base.find(token => token.startColumn <= startColumn && token.endColumn >= endColumn);
		const token = semantic ?? lexical;
		if (!token) continue;
		result.push(Object.freeze({
			startColumn,
			endColumn,
			...(token.presentation === undefined ? {} : { presentation: token.presentation }),
			...(token.modifiers === undefined ? {} : { modifiers: token.modifiers }),
			...(semantic || token.syntaxPresentation === undefined ? {} : { syntaxPresentation: token.syntaxPresentation }),
		}));
	}
	return Object.freeze(result);
}

function applySyntaxPresentation(element: HTMLElement, presentation: NonNullable<LanguageToken["presentation"]>): void {
	if (presentation.foreground !== undefined) element.style.color = presentation.foreground;
	if (presentation.background !== undefined) element.style.backgroundColor = presentation.background;
	if (presentation.fontStyle?.includes("italic")) element.style.fontStyle = "italic";
	if (presentation.fontStyle?.includes("bold")) element.style.fontWeight = "bold";
	const decorations = presentation.fontStyle?.filter(style => style === "underline" || style === "strikethrough").map(style => style === "strikethrough" ? "line-through" : style) ?? [];
	if (decorations.length > 0) element.style.textDecorationLine = decorations.join(" ");
}

/** Maps standard LSP modifier names to Stanza's closed browser presentation set. */
export function resolveStanzaSemanticTokenModifiers(token: LanguageToken): readonly SemanticTokenModifier[] {
	const resolved = new Set<SemanticTokenModifier>();
	for (const modifier of token.modifiers) {
		switch (modifier) {
			case "declaration":
			case "definition": resolved.add(SemanticTokenModifier.Declaration); break;
			case "readonly": resolved.add(SemanticTokenModifier.Readonly); break;
			case "static": resolved.add(SemanticTokenModifier.Static); break;
			case "deprecated": resolved.add(SemanticTokenModifier.Deprecated); break;
			case "abstract": resolved.add(SemanticTokenModifier.Abstract); break;
			case "async": resolved.add(SemanticTokenModifier.Async); break;
		}
	}
	return Object.freeze([...resolved]);
}
