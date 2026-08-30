import { h, reset, fragment as createFragment } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { DomReadingContext } from './domReadingContext.js';
import { RangeUtil } from './rangeUtil.js';
import { EditorTextDirection, type EditorViewLineOptions } from './viewLineOptions.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type Range } from '../../../common/core/range.js';
import { SemanticTokenModifier, SemanticTokenPresentation, type ResolvedSemanticToken, type SemanticTokenSource } from '../../../common/services/resolvedSemanticTokens.js';
import { type LanguageToken } from '../../../common/tokens/languageTokens.js';
import { CharacterMapping, DomPosition } from '../../../common/viewLayout/viewLineRenderer.js';
import { type FloatHorizontalRange } from '../../view/renderingContext.js';

interface BrowserCaretPosition {
	readonly offsetNode: Node;
	readonly offset: number;
}

interface BrowserCaretRange {
	readonly startContainer: Node;
	readonly startOffset: number;
}

interface BrowserCaretDocument {
	caretPositionFromPoint?(x: number, y: number): BrowserCaretPosition | null;
	caretRangeFromPoint?(x: number, y: number): BrowserCaretRange | null;
}

/** Owns one reusable virtual-line DOM subtree rendered by EditorViewLines. */
export class EditorViewLine {
	public static readonly CLASS_NAME = 'view-line';
	public readonly domNode: FastDomNode<HTMLDivElement>;
	public readonly textElement: HTMLSpanElement;
	private characterMapping: CharacterMapping;
	private renderedText = '';

	constructor(host: HTMLElement, lineIndex: number, private readonly options: EditorViewLineOptions) {
		const domNode = new FastDomNode(h(host.ownerDocument, "div"));
		const textElement = h(host.ownerDocument, "span");
		domNode.setClassName(EditorViewLine.CLASS_NAME);
		domNode.domNode.dataset.lineIndex = String(lineIndex);
		textElement.className = "stanza-editor-line-text";
		textElement.dir = options.textDirection;
		domNode.domNode.append(textElement);
		this.domNode = domNode;
		this.textElement = textElement;
		this.characterMapping = projectStanzaSemanticTokenLine(this.textElement, '', [], [], this.options.tabSize);
	}

	public hasTextOffset(offset: number): boolean {
		return Number.isSafeInteger(offset) && offset >= 0 && offset <= this.renderedText.length;
	}

	public renderText(text: string, tokens: readonly ResolvedSemanticToken[], brackets: readonly BracketColorizationSpan[]): void {
		this.characterMapping = projectStanzaSemanticTokenLine(this.textElement, text, tokens, brackets, this.options.tabSize);
		this.renderedText = text;
	}

	public layoutLine(lineHeight: number): void {
		this.domNode.setHeight(lineHeight);
		this.domNode.setLineHeight(lineHeight);
	}

	public getHorizontalRanges(startOffset: number, endOffset: number, context = this.createReadingContext()): readonly FloatHorizontalRange[] | undefined {
		if (!this.hasTextOffset(startOffset) || !this.hasTextOffset(endOffset) || endOffset < startOffset) {
			throw new RangeError('View line offsets must be ordered UTF-16 positions');
		}
		const start = this.characterMapping.getDomPosition(startOffset + 1);
		const end = this.characterMapping.getDomPosition(endOffset + 1);
		return RangeUtil.readHorizontalRanges(this.textElement, start.partIndex, start.charIndex, end.partIndex, end.charIndex, context) ?? undefined;
	}

	public getCaretLeft(offset: number): number | undefined {
		return this.getHorizontalRanges(offset, offset)?.[0]?.left;
	}

	public isRightToLeft(): boolean {
		if (this.options.textDirection === EditorTextDirection.RightToLeft) return true;
		if (this.options.textDirection === EditorTextDirection.LeftToRight) return false;
		return this.textElement.ownerDocument.defaultView?.getComputedStyle(this.textElement).direction === 'rtl';
	}

	public getOffsetAtClientPoint(clientX: number, clientY: number): number | undefined {
		if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) throw new RangeError('View line hit-test coordinates must be finite');
		const document = this.textElement.ownerDocument as unknown as BrowserCaretDocument;
		const position = document.caretPositionFromPoint?.(clientX, clientY) ?? document.caretRangeFromPoint?.(clientX, clientY);
		if (!position) return undefined;
		const node = 'offsetNode' in position ? position.offsetNode : position.startContainer;
		const offset = 'offsetNode' in position ? position.offset : position.startOffset;
		if (!this.textElement.contains(node)) return undefined;
		if (node === this.textElement) {
			const spanNode = (this.textElement.children[offset] ?? this.textElement.children[offset - 1]) as HTMLElement | undefined;
			if (!spanNode) return undefined;
			return this.getColumnOfNodeOffset(spanNode, spanNode === this.textElement.children[offset] ? 0 : spanNode.textContent?.length ?? 0);
		}
		const spanNode = node.nodeType === node.TEXT_NODE ? node.parentElement : node as HTMLElement;
		if (!spanNode) return undefined;
		const charOffset = node.nodeType === node.TEXT_NODE ? offset : offset === 0 ? 0 : spanNode.textContent?.length ?? 0;
		return this.getColumnOfNodeOffset(spanNode, charOffset);
	}

	public getColumnOfNodeOffset(spanNode: HTMLElement, offset: number): number | undefined {
		const partIndex = Array.prototype.indexOf.call(this.textElement.children, spanNode) as number;
		if (partIndex < 0 || !Number.isSafeInteger(offset) || offset < 0) return undefined;
		const partLength = spanNode.textContent?.length ?? 0;
		if (offset > partLength) return undefined;
		return this.characterMapping.getColumn(new DomPosition(partIndex, offset), partLength) - 1;
	}

	private createReadingContext(): DomReadingContext {
		return new DomReadingContext(this.domNode.domNode, this.textElement);
	}
}

export { SemanticTokenModifier, SemanticTokenPresentation } from '../../../common/services/resolvedSemanticTokens.js';
export type { ResolvedSemanticToken, SemanticTokenSource } from '../../../common/services/resolvedSemanticTokens.js';

export interface BracketColorizationSpan {
	readonly startColumn: number;
	readonly endColumn: number;
	readonly level: number;
}

export interface BracketGuide {
	readonly opening: Range;
	readonly closing: Range;
	readonly level: number;
}

/** Feature-neutral bracket projection consumed by the browser viewport. */
export interface BracketColorizationSource {
	readonly textModel: TextModel;
	getLineBrackets(lineIndex: number): readonly BracketColorizationSpan[];
	getBracketGuides?(startLineIndex: number, endLineIndexInclusive: number): readonly BracketGuide[];
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
	const boundaries = [...new Set([0, lineText.length, ...tokens.flatMap(token => [token.startColumn, token.endColumn]), ...brackets.flatMap(bracket => [bracket.startColumn, bracket.endColumn])])].sort((left, right) => left - right);
	const characterMapping = new CharacterMapping(lineText.length + 1, Math.max(1, boundaries.length - 1));
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
		validateLineTokens(source.textModel.getLineContent((line.lineIndex) + 1), tokens);
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

function applySyntaxPresentation(element: HTMLElement, presentation: NonNullable<LanguageToken["presentation"]>): void {
	if (presentation.foreground !== undefined) element.style.color = presentation.foreground;
	if (presentation.background !== undefined) element.style.backgroundColor = presentation.background;
	if (presentation.fontStyle?.includes("italic")) element.style.fontStyle = "italic";
	if (presentation.fontStyle?.includes("bold")) element.style.fontWeight = "bold";
	const decorations = presentation.fontStyle?.filter(style => style === "underline" || style === "strikethrough").map(style => style === "strikethrough" ? "line-through" : style) ?? [];
	if (decorations.length > 0) element.style.textDecorationLine = decorations.join(" ");
}
