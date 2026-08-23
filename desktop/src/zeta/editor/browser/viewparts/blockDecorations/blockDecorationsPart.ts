import './blockDecorations.css';
import { h } from '../../../../base/browser/dom.js';
import { type EditorViewportLayout } from '../../../common/viewLayout/editorViewportModel.js';
import { DecorationsPart } from '../decorations/decorationsPart.js';
import { EditorOverlayPart, EditorViewContext } from '../viewPart.js';
import { resolveAsterBlockDecorationGeometry } from './blockDecorationsProjection.js';

export class BlockDecorationsPart extends EditorOverlayPart {
	public readonly domNode: HTMLDivElement;

	private readonly decorations: DecorationsPart;
	private readonly blocks: HTMLDivElement[] = [];

	constructor(context: EditorViewContext, decorations: DecorationsPart, container: HTMLElement) {
		super(context);

		this.decorations = decorations;
		this.domNode = h(container.ownerDocument, 'div');
		this.domNode.className = 'aster-editor-block-decorations';
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
		container.append(this.domNode);
		this.defer(() => this.domNode.remove());
	}

	public render(layout: EditorViewportLayout): void {
		const context = this.context.overlayContext(layout);
		if (!context) {
			return;
		}

		this.domNode.style.width = `${layout.contentSize.width}px`;
		this.domNode.style.height = `${layout.contentSize.height}px`;

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
				block = h(this.domNode.ownerDocument, 'div');
				this.domNode.append(block);
				this.blocks.push(block);
			}

			const [paddingTop, , paddingBottom] = geometry.padding;
			block.className = `aster-editor-block-decoration ${presentation.className}`;
			block.dataset.decorationId = String(decoration.id);
			block.style.left = `${geometry.left}px`;
			block.style.width = `${geometry.width}px`;
			block.style.top = `${geometry.top - paddingTop}px`;
			block.style.height = `${geometry.bottom - geometry.top + paddingTop + paddingBottom}px`;

			count++;
		}

		for (let index = count; index < this.blocks.length; index++) {
			this.blocks[index]!.remove();
		}
		this.blocks.length = count;
	}
}
