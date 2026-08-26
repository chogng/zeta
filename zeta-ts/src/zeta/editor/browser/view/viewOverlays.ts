import { type Event } from '../../../base/common/event.js';
import { DisposableOwner } from '../../../base/common/lifecycle.js';
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
export class ViewOverlays extends DisposableOwner {
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
			options.model,
			options.decorationSources,
		));
		this.onDidChangeDecorations = this.decorationsPart.onDidChange;
		this.register(new LinesDecorationsPart(context, this.decorationsPart));
		this.blockDecorationsPart = this.register(new BlockDecorationsPart(
			context,
			this.decorationsPart,
			options.contentElement,
		));
		this.register(new MarginDecorationsPart(context, this.decorationsPart));
		this.register(new IndentGuidesPart(context, {
			showIndentationGuides: options.showIndentationGuides,
			tabSize: options.indentationTabSize,
		}));
		this.selectionsPart = this.register(new SelectionsPart(context, options.selectionController));
		this.viewCursorsPart = this.register(new ViewCursorsPart(context, options.selectionController));
		this.compositionPart = this.register(new CompositionPart(context, options.model));
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
		this.own(part);
		return part;
	}
}
