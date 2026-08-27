import { type Event } from '../../../base/common/event.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import { type EditorSelectionController } from '../../common/cursor/editorSelectionController.js';
import { type TextRange } from '../../common/core/text.js';
import { type TextModel } from '../../common/model/textModel.js';
import { type DecorationSource } from '../viewparts/decorations/decorationPresentation.js';
import { BlockDecorationsPart } from '../viewparts/blockDecorations/blockDecorationsPart.js';
import { CompositionPart } from '../viewparts/composition/compositionPart.js';
import { DecorationsPart } from '../viewparts/decorations/decorationsPart.js';
import { IndentGuidesPart } from '../viewparts/indentGuides/indentGuidesPart.js';
import { LinesDecorationsPart } from '../viewparts/linesDecorations/linesDecorationsPart.js';
import { MarginDecorationsPart } from '../viewparts/marginDecorations/marginDecorationsPart.js';
import { SelectionsPart } from '../viewparts/selections/selectionsPart.js';
import { ViewCursorsPart } from '../viewparts/viewCursors/viewCursorsPart.js';
import { DynamicViewOverlay } from './dynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from './viewPart.js';

export interface ViewOverlaysOptions {
	readonly contentElement: HTMLDivElement;
	readonly model: TextModel;
	readonly selectionController: EditorSelectionController | undefined;
	readonly decorationSources: readonly DecorationSource[];
	readonly showIndentationGuides: boolean;
	readonly indentationTabSize: number;
}

/** Coordinates row and block overlays while keeping their concrete projections independent. */
export class ViewOverlays extends Disposable {
	readonly domNodes: readonly HTMLElement[];
	readonly blockDecorationsPart: BlockDecorationsPart;
	readonly decorationsPart: DecorationsPart;
	readonly onDidChangeDecorations: Event<void>;

	private readonly parts: DynamicViewOverlay[] = [];
	private readonly selectionsPart: SelectionsPart;
	private readonly viewCursorsPart: ViewCursorsPart;
	private readonly compositionPart: CompositionPart;

	constructor(context: EditorViewContext, options: ViewOverlaysOptions) {
		super();
		this.decorationsPart = this.register(new DecorationsPart(
			context,
			options.contentElement,
			options.model,
			options.decorationSources,
		));
		this.onDidChangeDecorations = this.decorationsPart.onDidChange;
		const linesDecorationsPart = this.register(new LinesDecorationsPart(context, options.contentElement, this.decorationsPart, options.decorationSources));
		this.blockDecorationsPart = this.register(new BlockDecorationsPart(
			context,
			this.decorationsPart,
			options.contentElement,
		));
		const marginDecorationsPart = this.register(new MarginDecorationsPart(context, options.contentElement, this.decorationsPart));
		const indentGuidesPart = this.register(new IndentGuidesPart(context, {
			host: options.contentElement,
			showIndentationGuides: options.showIndentationGuides,
			tabSize: options.indentationTabSize,
		}));
		this.selectionsPart = this.register(new SelectionsPart(context, options.contentElement, options.selectionController));
		this.viewCursorsPart = this.register(new ViewCursorsPart(context, options.contentElement, options.selectionController));
		this.compositionPart = this.register(new CompositionPart(context, options.contentElement, options.model));
		this.domNodes = Object.freeze([
			indentGuidesPart.domNode,
			this.decorationsPart.domNode,
			this.selectionsPart.domNode,
			this.compositionPart.domNode,
			this.viewCursorsPart.domNode,
			linesDecorationsPart.domNode,
			marginDecorationsPart.domNode,
		]);
	}

	prepareRender(context: EditorRenderingContext): void {
		for (const part of this.parts) {
			part.prepareRender(context);
		}
	}

	render(context: EditorRenderingContext): void {
		for (const part of this.parts) {
			part.render(context);
		}
	}

	renderSelection(context: EditorRenderingContext): void {
		this.selectionsPart.renderNow(context);
		this.viewCursorsPart.renderNow(context);
	}

	setCompositionRange(range: TextRange | undefined): void {
		this.compositionPart.setRange(range);
	}

	private register<TPart extends DynamicViewOverlay>(part: TPart): TPart {
		this.parts.push(part);
		this._register(part);
		return part;
	}
}
