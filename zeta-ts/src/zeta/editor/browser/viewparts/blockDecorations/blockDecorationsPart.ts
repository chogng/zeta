import './blockDecorations.css';
import { h } from '../../../../base/browser/dom.js';
import { FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { type EditorViewportLayout } from '../../../common/viewLayout/editorViewportModel.js';
import { DecorationsPart } from '../decorations/decorationsPart.js';
import { EditorOverlayPart, EditorViewContext } from '../viewPart.js';
import { resolveAsterBlockDecorationGeometry } from './blockDecorationsProjection.js';

export class BlockDecorationsPart extends EditorOverlayPart {
	public readonly domNode: HTMLDivElement;

	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly decorations: DecorationsPart;
	private readonly blocks: FastDomNode<HTMLDivElement>[] = [];

	constructor(context: EditorViewContext, decorations: DecorationsPart, ownerDocument: Document) {
		super(context);

		this.decorations = decorations;
		this.domNode = this.adopt(h(ownerDocument, 'div'), domNode => domNode.remove());
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName('aster-editor-block-decorations');
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
	}

	public render(layout: EditorViewportLayout): void {
		const context = this.context.overlayContext(layout);
		if (!context) {
			return;
		}

		this.root.setWidth(layout.contentSize.width);
		this.root.setHeight(layout.contentSize.height);

		let count = 0;
		const decorations = this.decorations.visibleDecorations(context);
		for (const decoration of decorations) {
			const presentation = decoration.blockDecoration;
			if (!presentation) {
				continue;
			}

			const geometry = resolveAsterBlockDecorationGeometry(context, layout, decoration);
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
			block.setClassName(`aster-editor-block-decoration ${presentation.className}`);
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
