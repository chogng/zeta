import './blockDecorations.css';
import { h } from '../../../../base/browser/dom.js';
import { FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { DecorationsPart } from '../decorations/decorationsPart.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from '../../view/viewPart.js';
import { resolveStanzaBlockDecorationGeometry } from './blockDecorationsProjection.js';

export class BlockDecorationsPart extends DynamicViewOverlay {
	public readonly domNode: HTMLDivElement;

	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly decorations: DecorationsPart;
	private readonly blocks: FastDomNode<HTMLDivElement>[] = [];

	constructor(context: EditorViewContext, decorations: DecorationsPart, host: HTMLElement) {
		super(context);

		this.decorations = decorations;
		this.domNode = this.adopt(h(host.ownerDocument, 'div'), domNode => domNode.remove());
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
