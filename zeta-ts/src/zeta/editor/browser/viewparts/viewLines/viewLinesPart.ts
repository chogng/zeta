import "./viewLines.css";
import { h, reset, fragment as createFragment } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type EditorLineRange, type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorViewPart } from "../viewPart.js";
import { type BracketColorizationSource, type ResolvedSemanticToken, type SemanticTokenSource, projectStanzaSemanticTokenLine } from "../semanticTokens/semanticTokenPresentation.js";
import { type EditorLineGutterDecoration } from "../margin/lineGutterDecoration.js";
import { createStanzaRenderedLine, type RenderedLine } from "./renderedLine.js";

export type ViewLinesTextDirection = "auto" | "ltr" | "rtl";

export interface ViewLinesPartOptions {
	readonly host: HTMLElement;
	readonly model: TextModel;
	readonly readVisualProjection: () => EditorVisualLineProjection;
	readonly readProjectionRevision: () => number;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
	readonly bracketColorizationSource: BracketColorizationSource | undefined;
	readonly lineGutterDecoration: EditorLineGutterDecoration | undefined;
	readonly textDirection: ViewLinesTextDirection;
}

/** Owns the virtualized text rows and their semantic text projection. */
export class ViewLinesPart extends DisposableOwner implements EditorViewPart {
	readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly model: TextModel;
	private readonly readVisualProjection: () => EditorVisualLineProjection;
	private readonly readProjectionRevision: () => number;
	private readonly semanticTokenSource: SemanticTokenSource | undefined;
	private readonly bracketColorizationSource: BracketColorizationSource | undefined;
	private readonly lineGutterDecoration: EditorLineGutterDecoration | undefined;
	private readonly textDirection: ViewLinesTextDirection;
	private lines = new Map<number, RenderedLine>();
	private renderedRange: EditorLineRange = { startLineIndex: 0, endLineIndexExclusive: 0 };
	private renderedModelVersion = -1;
	private renderedLineHeight = -1;
	private renderedProjectionRevision = -1;

	constructor(options: ViewLinesPartOptions) {
		super();
		this.model = options.model;
		this.readVisualProjection = options.readVisualProjection;
		this.readProjectionRevision = options.readProjectionRevision;
		this.semanticTokenSource = options.semanticTokenSource;
		this.bracketColorizationSource = options.bracketColorizationSource;
		this.lineGutterDecoration = options.lineGutterDecoration;
		this.textDirection = options.textDirection;
		this.domNode = this.adopt(h(options.host.ownerDocument, "div"), domNode => domNode.remove());
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName("stanza-editor-lines");
	}

	get renderedLines(): ReadonlyMap<number, RenderedLine> {
		return this.lines;
	}

	render(layout: EditorViewportLayout): void {
		this.root.setTransform(`translate3d(0, ${layout.renderTop}px, 0)`);
		this.reconcileLines(layout);
	}

	/** Reprojects semantic tokens without rebuilding the visible row window. */
	renderVisibleLineText(): void {
		const semanticTokens = this.resolveSemanticTokenRange(this.renderedRange);
		const visualProjection = this.readVisualProjection();
		for (const [visualLineIndex, line] of this.lines) {
			const visualLine = visualProjection.lineAt(visualLineIndex);
			if (visualLine) this.projectLineText(line, visualLine, semanticTokens.get(visualLine.logicalLineIndex) ?? []);
		}
	}

	private reconcileLines(layout: EditorViewportLayout): void {
		const visualProjection = this.readVisualProjection();
		const projectionRevision = this.readProjectionRevision();
		if (visualProjection.modelVersion !== layout.modelVersion) return;
		if (
			this.renderedModelVersion === layout.modelVersion &&
			this.renderedLineHeight === layout.lineHeight &&
			this.renderedProjectionRevision === projectionRevision &&
			lineRangesEqual(this.renderedRange, layout.renderLines)
		) return;

		const semanticTokens = this.resolveSemanticTokenRange(layout.renderLines);
		const fragment = createFragment(this.domNode.ownerDocument);
		const next = new Map<number, RenderedLine>();
		for (let visualLineIndex = layout.renderLines.startLineIndex; visualLineIndex < layout.renderLines.endLineIndexExclusive; visualLineIndex += 1) {
			const visualLine = visualProjection.lineAt(visualLineIndex);
			if (!visualLine) throw new Error("Viewport render range exceeds the visual line projection");
			const existing = this.lines.get(visualLineIndex);
			const line = existing ?? createStanzaRenderedLine(this.domNode.ownerDocument, visualLineIndex, this.lineGutterDecoration);
			line.domNode.domNode.dataset.logicalLineIndex = String(visualLine.logicalLineIndex);
			if (!existing || this.renderedModelVersion !== layout.modelVersion || this.renderedProjectionRevision !== projectionRevision) {
				line.textElement.dir = this.textDirection;
				this.projectLineText(line, visualLine, semanticTokens.get(visualLine.logicalLineIndex) ?? []);
			}
			if (!existing || this.renderedLineHeight !== layout.lineHeight) {
				line.domNode.setHeight(layout.lineHeight);
				line.domNode.setLineHeight(layout.lineHeight);
			}
			next.set(visualLineIndex, line);
			fragment.append(line.domNode.domNode);
		}
		reset(this.domNode, fragment);
		this.lines = next;
		this.renderedRange = layout.renderLines;
		this.renderedModelVersion = layout.modelVersion;
		this.renderedLineHeight = layout.lineHeight;
		this.renderedProjectionRevision = projectionRevision;
	}

	private projectLineText(line: RenderedLine, visualLine: { readonly logicalLineIndex: number; readonly startColumn: number; readonly endColumn: number }, tokens: readonly ResolvedSemanticToken[]): void {
		const fullText = this.model.getLineContent(visualLine.logicalLineIndex);
		const text = fullText.slice(visualLine.startColumn, visualLine.endColumn);
		const brackets = this.bracketColorizationSource?.getLineBrackets(visualLine.logicalLineIndex) ?? [];
		projectStanzaSemanticTokenLine(
			line.textElement,
			text,
			clipSemanticTokens(tokens, visualLine.startColumn, visualLine.endColumn),
			clipBracketColorizations(brackets, visualLine.startColumn, visualLine.endColumn),
		);
	}

	private resolveSemanticTokenRange(range: EditorLineRange): ReadonlyMap<number, readonly ResolvedSemanticToken[]> {
		const source = this.semanticTokenSource;
		if (!source) return new Map();
		const tokens = new Map<number, readonly ResolvedSemanticToken[]>();
		const projection = this.readVisualProjection();
		for (let visualLineIndex = range.startLineIndex; visualLineIndex < range.endLineIndexExclusive; visualLineIndex += 1) {
			const visualLine = projection.lineAt(visualLineIndex);
			if (visualLine && !tokens.has(visualLine.logicalLineIndex)) tokens.set(visualLine.logicalLineIndex, source.getLineTokens(visualLine.logicalLineIndex));
		}
		return tokens;
	}
}

function lineRangesEqual(left: EditorLineRange, right: EditorLineRange): boolean {
	return left.startLineIndex === right.startLineIndex && left.endLineIndexExclusive === right.endLineIndexExclusive;
}

function clipSemanticTokens(tokens: readonly ResolvedSemanticToken[], startColumn: number, endColumn: number): readonly ResolvedSemanticToken[] {
	return Object.freeze(tokens.flatMap(token => {
		const start = Math.max(token.startColumn, startColumn);
		const end = Math.min(token.endColumn, endColumn);
		if (end <= start) return [];
		return [Object.freeze({
			startColumn: start - startColumn,
			endColumn: end - startColumn,
			presentation: token.presentation,
			...(token.modifiers && token.modifiers.length > 0 ? { modifiers: token.modifiers } : {}),
		})];
	}));
}

function clipBracketColorizations(brackets: readonly { readonly startColumn: number; readonly endColumn: number; readonly level: number }[], startColumn: number, endColumn: number): readonly { readonly startColumn: number; readonly endColumn: number; readonly level: number }[] {
	return Object.freeze(brackets.flatMap(bracket => {
		const start = Math.max(bracket.startColumn, startColumn);
		const end = Math.min(bracket.endColumn, endColumn);
		if (end <= start) return [];
		return [Object.freeze({ startColumn: start - startColumn, endColumn: end - startColumn, level: bracket.level })];
	}));
}
