import './glyphMargin.css';
import { h } from '../../../../base/browser/dom.js';
import { FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { appendIcon } from '../../../../base/browser/ui/icon/icon.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { type TextDecorationId } from '../../../common/model/decorationCollection.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { EditorViewPart, type EditorRenderingContext } from '../../view/viewPart.js';
import { type DecorationGlyphMarginPresentation, type DecorationSource, type ResolvedDecoration, GlyphMarginLane } from '../decorations/decorationPresentation.js';
import { type DecorationsPart } from '../decorations/decorationsPart.js';

export const EDITOR_GLYPH_MARGIN_LANE_WIDTH = 20;

export interface GlyphMarginPartOptions {
	readonly host: HTMLElement;
	readonly sources: readonly DecorationSource[];
	readonly decorationsPart: DecorationsPart;
	readonly readVisualLines: () => EditorVisualLineProjection;
	readonly readLeft: () => number;
}

/** Renders decoration-backed glyphs in shared, stable margin lanes. */
export class GlyphMarginPart extends EditorViewPart {
	public readonly domNode: HTMLDivElement;
	public readonly width: number;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly decorationsPart: DecorationsPart;
	private readonly readVisualLines: () => EditorVisualLineProjection;
	private readonly readLeft: () => number;
	private readonly laneDomNodes: ReadonlyMap<GlyphMarginLane, HTMLSpanElement>;
	private readonly buttons = new Map<TextDecorationId, HTMLButtonElement>();

	constructor(options: GlyphMarginPartOptions) {
		super();
		this.decorationsPart = options.decorationsPart;
		this.readVisualLines = options.readVisualLines;
		this.readLeft = options.readLeft;
		const lanes = collectGlyphMarginLanes(options.sources);
		this.width = lanes.length * EDITOR_GLYPH_MARGIN_LANE_WIDTH;
		this.domNode = h(options.host.ownerDocument, 'div');
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName('stanza-editor-glyph-margin');
		this.root.setWidth(this.width);
		this.root.setHidden(lanes.length === 0);
		this.laneDomNodes = new Map(lanes.map((lane, index) => {
			const domNode = h(this.domNode.ownerDocument, 'span');
			domNode.className = 'stanza-editor-glyph-margin-lane';
			domNode.dataset.glyphMarginLane = lane;
			domNode.style.left = `${index * EDITOR_GLYPH_MARGIN_LANE_WIDTH}px`;
			this.domNode.append(domNode);
			return [lane, domNode] as const;
		}));
		this._register(toDisposable(() => this.domNode.remove()));
	}

	public render(context: EditorRenderingContext): void {
		this.root.setLeft(context.layout.scrollPosition.left + this.readLeft());
		this.root.setHeight(context.layout.contentSize.height);
		for (const laneDomNode of this.laneDomNodes.values()) laneDomNode.style.height = `${context.layout.contentSize.height}px`;
		const overlay = context.overlay;
		if (!overlay) {
			this.clearButtons();
			return;
		}
		const visualLines = this.readVisualLines();
		const visible = this.decorationsPart.visibleDecorations(overlay)
			.filter((decoration): decoration is ResolvedDecoration & { readonly glyphMargin: DecorationGlyphMarginPresentation } => decoration.glyphMargin !== undefined)
			.sort(compareGlyphDecorations);
		const renderedIds = new Set<TextDecorationId>();
		for (const decoration of visible) {
			const visualLineIndex = visualLines.visualLineIndexAt(decoration.range.start);
			if (visualLineIndex < context.viewportData.startLineIndex || visualLineIndex >= context.viewportData.endLineIndexExclusive) continue;
			const laneDomNode = this.laneDomNodes.get(decoration.glyphMargin.lane);
			if (!laneDomNode) continue;
			const button = this.buttons.get(decoration.id) ?? this.createButton(decoration.id, laneDomNode);
			this.renderButton(button, decoration, context.viewportData.getLineTop(visualLineIndex), context.viewportData.lineHeight);
			renderedIds.add(decoration.id);
		}
		for (const [id, button] of this.buttons) {
			if (renderedIds.has(id)) continue;
			button.remove();
			this.buttons.delete(id);
		}
	}

	private createButton(id: TextDecorationId, laneDomNode: HTMLElement): HTMLButtonElement {
		const button = h(this.domNode.ownerDocument, 'button');
		button.type = 'button';
		button.dataset.decorationId = String(id);
		laneDomNode.append(button);
		this.buttons.set(id, button);
		return button;
	}

	private renderButton(button: HTMLButtonElement, decoration: ResolvedDecoration & { readonly glyphMargin: DecorationGlyphMarginPresentation }, top: number, height: number): void {
		const glyph = decoration.glyphMargin;
		button.className = `stanza-editor-glyph-margin-decoration${glyph.className ? ` ${glyph.className}` : ''}`;
		button.dataset.decorationOwner = glyph.owner;
		button.dataset.logicalLineIndex = String(decoration.range.start.lineIndex);
		button.setAttribute('aria-label', glyph.ariaLabel);
		setOptionalBooleanAttribute(button, 'aria-expanded', glyph.expanded);
		setOptionalBooleanAttribute(button, 'aria-pressed', glyph.pressed);
		if (glyph.title === undefined) button.removeAttribute('title');
		else button.title = glyph.title;
		button.style.top = `${top}px`;
		button.style.height = `${height}px`;
		button.style.zIndex = String(glyph.zIndex ?? 0);
		if (button.dataset.iconId === glyph.icon?.id) return;
		button.replaceChildren();
		if (glyph.icon) {
			appendIcon(glyph.icon, button);
			button.dataset.iconId = glyph.icon.id;
		} else {
			delete button.dataset.iconId;
		}
	}

	private clearButtons(): void {
		for (const button of this.buttons.values()) button.remove();
		this.buttons.clear();
	}
}

export function collectGlyphMarginLanes(sources: readonly DecorationSource[]): readonly GlyphMarginLane[] {
	const laneOwners = new Map<string, GlyphMarginLane>();
	for (const source of sources) {
		for (const definition of source.glyphMarginLanes) {
			const existing = laneOwners.get(definition.owner);
			if (existing !== undefined) throw new RangeError(`Duplicate glyph margin owner '${definition.owner}'`);
			laneOwners.set(definition.owner, definition.lane);
		}
	}
	return Object.freeze([...new Set(laneOwners.values())].sort((left, right) => laneOrder(left) - laneOrder(right)));
}

function compareGlyphDecorations(left: ResolvedDecoration & { readonly glyphMargin: DecorationGlyphMarginPresentation }, right: ResolvedDecoration & { readonly glyphMargin: DecorationGlyphMarginPresentation }): number {
	return left.range.start.lineIndex - right.range.start.lineIndex
		|| laneOrder(left.glyphMargin.lane) - laneOrder(right.glyphMargin.lane)
		|| (left.glyphMargin.zIndex ?? 0) - (right.glyphMargin.zIndex ?? 0)
		|| left.id - right.id;
}

function laneOrder(lane: GlyphMarginLane): number {
	switch (lane) {
		case GlyphMarginLane.Left: return 0;
		case GlyphMarginLane.Center: return 1;
		case GlyphMarginLane.Right: return 2;
	}
}

function setOptionalBooleanAttribute(element: HTMLElement, name: string, value: boolean | undefined): void {
	if (value === undefined) element.removeAttribute(name);
	else element.setAttribute(name, String(value));
}
