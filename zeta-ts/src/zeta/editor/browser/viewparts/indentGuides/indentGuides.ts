import "./indentGuides.css";
import { h } from "../../../../base/browser/dom.js";
import { type InternalGuidesOptions } from '../../../common/config/editorOptions.js';
import { type TextPosition } from '../../../common/core/text.js';
import { type EditorSelectionController } from '../../../common/cursor/cursor.js';
import { type EditorVisualLine } from '../../../common/viewModel/modelLineProjection.js';
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewLayer.js';
import { type BracketColorizationSource, type BracketGuide } from '../viewLines/viewLine.js';

interface IndentGuidesOptions {
	readonly host: HTMLElement;
	readonly guides: InternalGuidesOptions;
	readonly tabSize: number;
	readonly bracketColorizationSource: BracketColorizationSource | undefined;
	readonly selectionController: EditorSelectionController | undefined;
}

/** Owns and projects the visible indentation-guide rows. */
export class IndentGuidesOverlay extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly guides: InternalGuidesOptions;
	private readonly tabSize: number;
	private readonly bracketColorizationSource: BracketColorizationSource | undefined;
	private readonly selectionController: EditorSelectionController | undefined;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, options: IndentGuidesOptions) {
		super(context);
		this.rows = this._register(new ViewPartRows(options.host, 'stanza-editor-indent-guides-layer', 'stanza-editor-line-indent-guides'));
		this.domNode = this.rows.domNode;
		this.guides = options.guides;
		this.tabSize = options.tabSize;
		this.bracketColorizationSource = options.bracketColorizationSource;
		this.selectionController = options.selectionController;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		const rows = this.rows.render(context);
		const bracketGuides = this.resolveBracketGuides(context);
		const activeBracketGuide = this.resolveActiveBracketGuide(bracketGuides);
		const activeIndentation = this.resolveActiveIndentation(activeBracketGuide);
		for (const [visualLineIndex, row] of rows) {
			row.replaceChildren();
			const visualLine = overlay.visualLineProjection.lineAt(visualLineIndex);
			if (!visualLine) continue;
			const text = overlay.model.getLineContent(visualLine.logicalLineIndex);
			if (this.guides.indentation && visualLine.firstForLogicalLine) {
				for (const guide of createStanzaIndentationGuides(text, this.tabSize)) {
					const element = h(overlay.ownerDocument, "span");
					element.className = "stanza-editor-indent-guide";
					element.dataset.indentLevel = String(guide.level);
					element.style.left = `${overlay.textLeft + overlay.textMeasurer.measureLineWidth(text.slice(0, guide.columnIndex)) - 1}px`;
					if (activeIndentation?.level === guide.level && activeIndentation.startLineIndex <= visualLine.logicalLineIndex && visualLine.logicalLineIndex <= activeIndentation.endLineIndex) element.classList.add('active');
					row.append(element);
				}
			}
			for (const guide of bracketGuides) this.appendBracketGuide(row, visualLine, context.layout.lineHeight, guide, activeBracketGuide, overlay.textLeft, overlay.textMeasurer.measureLineWidth.bind(overlay.textMeasurer));
		}
	}

	private resolveBracketGuides(context: EditorRenderingContext): readonly BracketGuide[] {
		if (this.guides.bracketPairs === false || !this.bracketColorizationSource?.getBracketGuides || !context.overlay) return Object.freeze([]);
		const projection = context.overlay.visualLineProjection;
		const first = projection.lineAt(context.layout.renderLines.startLineIndex);
		const last = projection.lineAt(context.layout.renderLines.endLineIndexExclusive - 1);
		if (!first || !last) return Object.freeze([]);
		return this.bracketColorizationSource.getBracketGuides(first.logicalLineIndex, last.logicalLineIndex);
	}

	private resolveActiveBracketGuide(guides: readonly BracketGuide[]): BracketGuide | undefined {
		const position = this.selectionController?.selections.primary.active;
		if (!position) return undefined;
		return guides.filter(guide => containsPosition(guide, position)).sort(compareInnermostFirst)[0];
	}

	private resolveActiveIndentation(activeBracketGuide: BracketGuide | undefined): ActiveIndentationGuide | undefined {
		const highlight = this.guides.highlightActiveIndentation;
		if (highlight === false || (highlight !== 'always' && activeBracketGuide)) return undefined;
		const lineIndex = this.selectionController?.selections.primary.active.lineIndex;
		if (lineIndex === undefined) return undefined;
		const model = this.bracketColorizationSource?.textModel ?? this.selectionController!.textModel;
		const level = createStanzaIndentationGuides(model.getLineContent(lineIndex), this.tabSize).at(-1)?.level;
		if (!level) return undefined;
		let startLineIndex = lineIndex;
		let endLineIndex = lineIndex;
		while (startLineIndex > 0 && indentationLevel(model.getLineContent(startLineIndex - 1), this.tabSize) >= level) startLineIndex -= 1;
		while (endLineIndex + 1 < model.lineCount && indentationLevel(model.getLineContent(endLineIndex + 1), this.tabSize) >= level) endLineIndex += 1;
		return { level, startLineIndex, endLineIndex };
	}

	private appendBracketGuide(
		row: HTMLElement,
		visualLine: EditorVisualLine,
		lineHeight: number,
		guide: BracketGuide,
		activeGuide: BracketGuide | undefined,
		textLeft: number,
		measureLineWidth: (text: string) => number,
	): void {
		const lineIndex = visualLine.logicalLineIndex;
		if (lineIndex < guide.opening.start.lineIndex || lineIndex > guide.closing.start.lineIndex) return;
		if (lineIndex === guide.opening.start.lineIndex && visualLine.endColumn <= guide.opening.start.columnIndex) return;
		if (lineIndex === guide.closing.start.lineIndex && visualLine.startColumn > guide.closing.start.columnIndex) return;
		const active = activeGuide === guide;
		if (this.guides.bracketPairs === 'active' && !active) return;
		const openingLine = this.bracketColorizationSource!.textModel.getLineContent(guide.opening.start.lineIndex);
		const left = textLeft + measureLineWidth(openingLine.slice(0, guide.opening.start.columnIndex));
		const vertical = h(row.ownerDocument, 'span');
		vertical.className = 'stanza-editor-bracket-guide';
		vertical.dataset.bracketLevel = String(guide.level);
		vertical.style.left = `${left}px`;
		const openingVisualLine = lineIndex === guide.opening.start.lineIndex && visualLine.startColumn <= guide.opening.start.columnIndex && guide.opening.start.columnIndex < visualLine.endColumn;
		const closingVisualLine = lineIndex === guide.closing.start.lineIndex && visualLine.startColumn <= guide.closing.start.columnIndex && guide.closing.start.columnIndex <= visualLine.endColumn;
		if (openingVisualLine) vertical.style.top = `${lineHeight / 2}px`;
		if (closingVisualLine) vertical.style.bottom = `${lineHeight / 2}px`;
		if (active && this.guides.highlightActiveBracketPair) vertical.classList.add('active');
		row.append(vertical);
		const horizontalMode = this.guides.bracketPairsHorizontal;
		if (!closingVisualLine || horizontalMode === false || (horizontalMode === 'active' && !active)) return;
		const closingLine = this.bracketColorizationSource!.textModel.getLineContent(guide.closing.start.lineIndex);
		const closingLeft = textLeft + measureLineWidth(closingLine.slice(0, guide.closing.start.columnIndex));
		const horizontal = h(row.ownerDocument, 'span');
		horizontal.className = 'stanza-editor-bracket-guide-horizontal';
		horizontal.style.left = `${Math.min(left, closingLeft)}px`;
		horizontal.style.width = `${Math.abs(closingLeft - left)}px`;
		horizontal.style.top = `${lineHeight / 2}px`;
		if (active && this.guides.highlightActiveBracketPair) horizontal.classList.add('active');
		row.append(horizontal);
	}
}

interface ActiveIndentationGuide {
	readonly level: number;
	readonly startLineIndex: number;
	readonly endLineIndex: number;
}

function containsPosition(guide: BracketGuide, position: TextPosition): boolean {
	return guide.opening.start.compareTo(position) <= 0 && guide.closing.end.compareTo(position) >= 0;
}

function compareInnermostFirst(left: BracketGuide, right: BracketGuide): number {
	const opening = right.opening.start.compareTo(left.opening.start);
	return opening !== 0 ? opening : left.closing.end.compareTo(right.closing.end);
}

function indentationLevel(text: string, tabSize: number): number {
	return createStanzaIndentationGuides(text, tabSize).at(-1)?.level ?? 0;
}

export interface IndentationGuide {
	readonly columnIndex: number;
	readonly level: number;
}

/** Returns one guide at every complete visual indentation unit in leading whitespace. */
export function createStanzaIndentationGuides(text: string, tabSize: number): readonly IndentationGuide[] {
	if (typeof text !== "string") throw new TypeError("Stanza indentation guides require text");
	if (!Number.isSafeInteger(tabSize) || tabSize < 1) throw new RangeError("Stanza indentation guide tab size must be a positive safe integer");
	const guides: IndentationGuide[] = [];
	let visualColumn = 0;
	for (let columnIndex = 0; columnIndex < text.length; columnIndex += 1) {
		const character = text[columnIndex]!;
		if (character !== " " && character !== "\t") break;
		visualColumn = character === "\t"
			? visualColumn + tabSize - (visualColumn % tabSize)
			: visualColumn + 1;
		if (visualColumn % tabSize === 0) {
			guides.push(Object.freeze({
				columnIndex: columnIndex + 1,
				level: visualColumn / tabSize,
			}));
		}
	}
	return Object.freeze(guides);
}
