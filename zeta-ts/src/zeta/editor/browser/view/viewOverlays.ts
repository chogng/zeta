import { type Event } from '../../../base/common/event.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import { type EditorSelectionController } from '../../common/cursor/editorSelectionController.js';
import { type TextRange } from '../../common/core/text.js';
import { type TextModel } from '../../common/model/textModel.js';
import { type DecorationSource } from '../viewparts/decorations/decorations.js';
import { BlockDecorations } from '../viewparts/blockDecorations/blockDecorations.js';
import { DecorationsOverlay } from '../viewparts/decorations/decorations.js';
import { CurrentLineHighlightOverlay } from '../viewparts/currentLineHighlight/currentLineHighlight.js';
import { IndentGuidesOverlay } from '../viewparts/indentGuides/indentGuides.js';
import { LinesDecorationsOverlay } from '../viewparts/linesDecorations/linesDecorations.js';
import { MarginViewLineDecorationsOverlay } from '../viewparts/marginDecorations/marginDecorations.js';
import { SelectionsOverlay } from '../viewparts/selections/selections.js';
import { ViewCursors } from '../viewparts/viewCursors/viewCursors.js';
import { WhitespaceOverlay, type WhitespaceRenderingMode } from '../viewparts/whitespace/whitespace.js';
import { DynamicViewOverlay } from './dynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from './viewPart.js';

export interface ViewOverlaysOptions {
	readonly contentElement: HTMLDivElement;
	readonly model: TextModel;
	readonly selectionController: EditorSelectionController | undefined;
	readonly decorationSources: readonly DecorationSource[];
	readonly showIndentationGuides: boolean;
	readonly indentationTabSize: number;
	readonly renderWhitespace: WhitespaceRenderingMode;
}

/** Coordinates row and block overlays while keeping their concrete projections independent. */
export class ViewOverlays extends Disposable {
	readonly domNodes: readonly HTMLElement[];
	readonly blockDecorations: BlockDecorations;
	readonly decorations: DecorationsOverlay;
	readonly onDidChangeDecorations: Event<void>;

	private readonly parts: DynamicViewOverlay[] = [];
	private readonly selections: SelectionsOverlay;
	private readonly currentLineHighlight: CurrentLineHighlightOverlay;
	private readonly viewCursors: ViewCursors;

	constructor(context: EditorViewContext, options: ViewOverlaysOptions) {
		super();
		this.decorations = this.register(new DecorationsOverlay(
			context,
			options.contentElement,
			options.model,
			options.decorationSources,
		));
		this.onDidChangeDecorations = this.decorations.onDidChange;
		const linesDecorations = this.register(new LinesDecorationsOverlay(context, options.contentElement, this.decorations, options.decorationSources));
		this.blockDecorations = this.register(new BlockDecorations(
			context,
			this.decorations,
			options.contentElement,
		));
		const marginDecorations = this.register(new MarginViewLineDecorationsOverlay(context, options.contentElement, this.decorations));
		const indentGuides = this.register(new IndentGuidesOverlay(context, {
			host: options.contentElement,
			showIndentationGuides: options.showIndentationGuides,
			tabSize: options.indentationTabSize,
		}));
		const whitespace = this.register(new WhitespaceOverlay(context, options.contentElement, options.model, options.renderWhitespace));
		this.currentLineHighlight = this.register(new CurrentLineHighlightOverlay(context, options.contentElement, options.selectionController));
		this.selections = this.register(new SelectionsOverlay(context, options.contentElement, options.selectionController));
		this.viewCursors = this.register(new ViewCursors(context, options.contentElement, options.model, options.selectionController));
		this.domNodes = Object.freeze([
			indentGuides.domNode,
			whitespace.domNode,
			this.decorations.domNode,
			this.currentLineHighlight.domNode,
			this.selections.domNode,
			this.viewCursors.domNode,
			linesDecorations.domNode,
			marginDecorations.domNode,
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
		this.currentLineHighlight.renderNow(context);
		this.selections.renderNow(context);
		this.viewCursors.renderNow(context);
	}

	setCompositionRange(range: TextRange | undefined): void {
		this.viewCursors.setCompositionRange(range);
	}

	private register<TPart extends DynamicViewOverlay>(part: TPart): TPart {
		this.parts.push(part);
		this._register(part);
		return part;
	}
}
