import './blockDecorations.css';
import { h } from '../../../../base/browser/dom.js';
import { FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { EditorDecorationsOverlay } from '../decorations/decorations.js';
import { type ResolvedDecoration } from '../decorations/decorations.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type EditorViewportLayout } from '../../../common/viewLayout/viewLayout.js';
import { type EditorOverlayContext } from '../../view/renderingContext.js';
import { EditorDynamicViewOverlay } from '../../view/editorDynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from '../../view/viewPart.js';

export class EditorBlockDecorations extends EditorDynamicViewOverlay {
	public readonly domNode: HTMLDivElement;

	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly decorations: EditorDecorationsOverlay;
	private readonly blocks: FastDomNode<HTMLDivElement>[] = [];

	constructor(context: EditorViewContext, decorations: EditorDecorationsOverlay, host: HTMLElement) {
		super(context);

		this.decorations = decorations;
		const domNode = h(host.ownerDocument, 'div');
		this._register(toDisposable(() => domNode.remove()));
		this.domNode = domNode;
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName('stanza-editor-block-decorations');
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		const layout = context.layout;

		this.root.setWidth(layout.contentSize.width);
		this.root.setHeight(layout.contentSize.height);

		let count = 0;
		const decorations = this.decorations.visibleDecorations(overlay);
		for (const decoration of decorations) {
			const presentation = decoration.blockDecoration;
			if (!presentation) {
				continue;
			}

			const geometry = resolveStanzaBlockDecorationGeometry(overlay, layout, decoration);
			if (!geometry) {
				continue;
			}

			let block = this.blocks[count];
			if (!block) {
				block = new FastDomNode(h(this.domNode.ownerDocument, 'div'));
				this.domNode.append(block.domNode);
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
	context: EditorOverlayContext,
	layout: EditorViewportLayout,
	decoration: ResolvedDecoration,
): BlockDecorationGeometry | undefined {
	const presentation = decoration.blockDecoration;
	if (!presentation) return undefined;
	const projection = context.visualLineProjection;
	const startVisualLineIndex = firstVisualLineIndex(projection, decoration.range.startLineNumber - 1);
	if (startVisualLineIndex === undefined) return undefined;

	const lineTop = createLineTopReader(layout);
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
	const contentLeft = context.textLeft;
	return Object.freeze({
		top,
		bottom,
		left: contentLeft - padding[3],
		width: Math.max(0, layout.contentSize.width - contentLeft) + padding[1] + padding[3],
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

function createLineTopReader(layout: EditorViewportLayout): (visualLineIndex: number) => number {
	const offsets = layout.relativeVerticalOffset;
	if (offsets) {
		return visualLineIndex => {
			const offsetIndex = visualLineIndex - layout.renderLines.startLineIndex;
			if (offsetIndex >= 0 && offsetIndex < offsets.length) return offsets[offsetIndex]!;
			if (offsetIndex === offsets.length && offsets.length > 0) return offsets[offsets.length - 1]! + layout.lineHeight;
			return layout.renderTop + offsetIndex * layout.lineHeight;
		};
	}
	const paddingTop = layout.renderTop - layout.renderLines.startLineIndex * layout.lineHeight;
	return visualLineIndex => paddingTop + visualLineIndex * layout.lineHeight;
}
