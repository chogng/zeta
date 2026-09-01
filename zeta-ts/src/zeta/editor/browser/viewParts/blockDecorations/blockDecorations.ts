import './blockDecorations.css';
import { h } from '../../../../base/browser/dom.js';
import { FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { EditorOption } from '../../../common/config/editorOptions.js';
import * as viewEvents from '../../../common/viewEvents.js';
import { DecorationsOverlay } from '../decorations/decorations.js';
import { type ResolvedDecoration } from '../decorations/decorations.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type RenderingContext, type RestrictedRenderingContext } from '../../view/renderingContext.js';
import { ViewPart } from '../../view/viewPart.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
export class BlockDecorations extends ViewPart {
	public readonly domNode: FastDomNode<HTMLElement>;

	private readonly decorations: DecorationsOverlay;
	private readonly blocks: FastDomNode<HTMLDivElement>[] = [];

	constructor(context: ViewContext, decorations: DecorationsOverlay, host: HTMLElement, private readonly readVisualProjection: () => EditorVisualLineProjection, private readonly readTextLeft: () => number) {
		super(context);

		this.decorations = decorations;
		const domNode = h(host.ownerDocument, 'div');
		this._register(toDisposable(() => domNode.remove()));
		this.domNode = new FastDomNode(domNode);
		this.domNode.setClassName('stanza-editor-block-decorations');
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
	}

	public override onConfigurationChanged(event: viewEvents.ViewConfigurationChangedEvent): boolean {
		return event.hasChanged(EditorOption.layoutInfo);
	}

	public override onScrollChanged(event: viewEvents.ViewScrollChangedEvent): boolean {
		return event.scrollTopChanged || event.scrollHeightChanged || event.scrollWidthChanged;
	}

	public override onDecorationsChanged(_event: viewEvents.ViewDecorationsChangedEvent): boolean {
		return true;
	}

	public override onZonesChanged(_event: viewEvents.ViewZonesChangedEvent): boolean {
		return true;
	}

	public override prepareRender(_context: RenderingContext): void {
	}

	public render(context: RestrictedRenderingContext): void {
		this.domNode.setWidth(context.scrollWidth);
		this.domNode.setHeight(context.scrollHeight);

		let count = 0;
		const decorations = this.decorations.visibleDecorations(context);
		for (const decoration of decorations) {
			const presentation = decoration.blockDecoration;
			if (!presentation) {
				continue;
			}

			const geometry = resolveStanzaBlockDecorationGeometry(context, this.readVisualProjection(), this.readTextLeft(), decoration);
			if (!geometry) {
				continue;
			}

			let block = this.blocks[count];
			if (!block) {
				block = new FastDomNode(h(this.domNode.domNode.ownerDocument, 'div'));
				this.domNode.appendChild(block);
				this.blocks.push(block);
			}

			const [paddingTop, , paddingBottom] = geometry.padding;
			block.setClassName(`stanza-editor-block-decoration ${presentation.className}`);
			block.domNode.dataset.decorationId = String(decoration.id);
			block.setLeft(geometry.left);
			block.setWidth(geometry.width);
			block.setTop(geometry.top - paddingTop);
			block.setHeight(geometry.bottom - geometry.top + paddingTop + paddingBottom);

			count++;
		}

		for (let index = count; index < this.blocks.length; index++) {
			this.blocks[index]!.domNode.remove();
		}
		this.blocks.length = count;
	}
}

interface BlockDecorationGeometry {
	readonly top: number;
	readonly bottom: number;
	readonly left: number;
	readonly width: number;
	readonly padding: readonly [number, number, number, number];
}

function resolveStanzaBlockDecorationGeometry(
	context: RestrictedRenderingContext,
	projection: EditorVisualLineProjection,
	textLeft: number,
	decoration: ResolvedDecoration,
): BlockDecorationGeometry | undefined {
	const presentation = decoration.blockDecoration;
	if (!presentation) return undefined;
	const startVisualLineIndex = firstVisualLineIndex(projection, decoration.range.startLineNumber - 1);
	if (startVisualLineIndex === undefined) return undefined;

	const lineTop = (visualLineIndex: number): number => visualLineIndex >= projection.visualLineCount
		? context.getVerticalOffsetAfterLineNumber(projection.visualLineCount)
		: context.getVerticalOffsetForLineNumber(visualLineIndex + 1);
	let top: number;
	let bottom: number;
	if (presentation.isAfterEnd) {
		const endVisualLineIndex = lastVisualLineIndex(projection, decoration.range.endLineNumber - 1);
		if (endVisualLineIndex === undefined) return undefined;
		top = lineTop(endVisualLineIndex + 1);
		bottom = top;
	} else {
		const endLogicalLineIndex = lastLogicalLineIndex(decoration);
		const endVisualLineIndex = lastVisualLineIndex(projection, endLogicalLineIndex);
		if (endVisualLineIndex === undefined) return undefined;
		top = lineTop(startVisualLineIndex);
		bottom = decoration.range.isEmpty() && !presentation.doesNotCollapse ? top : lineTop(endVisualLineIndex + 1);
	}

	const padding = presentation.padding ?? [0, 0, 0, 0];
	const contentLeft = textLeft;
	return Object.freeze({
		top,
		bottom,
		left: contentLeft - padding[3],
		width: Math.max(0, context.scrollWidth - contentLeft) + padding[1] + padding[3],
		padding,
	});
}

function lastLogicalLineIndex(decoration: ResolvedDecoration): number {
	const { startLineNumber, endLineNumber, endColumn } = decoration.range;
	return endColumn === 1 && endLineNumber > startLineNumber ? endLineNumber - 2 : endLineNumber - 1;
}

function firstVisualLineIndex(projection: EditorVisualLineProjection, logicalLineIndex: number): number | undefined {
	if (logicalLineIndex < 0 || logicalLineIndex >= projection.logicalLineCount) return undefined;
	const visualLineIndex = projection.firstVisualLineIndex(logicalLineIndex);
	return projection.lineAt(visualLineIndex)?.logicalLineIndex === logicalLineIndex ? visualLineIndex : undefined;
}

function lastVisualLineIndex(projection: EditorVisualLineProjection, logicalLineIndex: number): number | undefined {
	const first = firstVisualLineIndex(projection, logicalLineIndex);
	if (first === undefined) return undefined;
	let last = first;
	for (let visualLineIndex = first + 1; visualLineIndex < projection.visualLineCount; visualLineIndex += 1) {
		if (projection.lineAt(visualLineIndex)?.logicalLineIndex !== logicalLineIndex) break;
		last = visualLineIndex;
	}
	return last;
}
