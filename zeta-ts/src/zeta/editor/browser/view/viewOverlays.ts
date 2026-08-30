import { type Event } from '../../../base/common/event.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import { type CursorsController } from '../../common/cursor/cursor.js';
import { type CursorChangeReason } from '../../common/cursorEvents.js';
import { type InternalGuidesOptions, type TextEditorCursorBlinkingStyle, type TextEditorCursorStyle } from '../../common/config/editorOptions.js';
import { type BareFontInfo } from '../../common/config/fontInfo.js';
import { type Range } from '../../common/core/range.js';
import { type TextModel } from '../../common/model/textModel.js';
import { type SemanticTokenSource } from '../../common/services/resolvedSemanticTokens.js';
import { type BracketColorizationSource } from '../viewParts/viewLines/viewLine.js';
import { type DecorationSource } from '../viewParts/decorations/decorations.js';
import { BlockDecorations } from '../viewParts/blockDecorations/blockDecorations.js';
import { DecorationsOverlay } from '../viewParts/decorations/decorations.js';
import { CurrentLineHighlightOverlay } from '../viewParts/currentLineHighlight/currentLineHighlight.js';
import { IndentGuidesOverlay } from '../viewParts/indentGuides/indentGuides.js';
import { LinesDecorationsOverlay } from '../viewParts/linesDecorations/linesDecorations.js';
import { MarginViewLineDecorationsOverlay } from '../viewParts/marginDecorations/marginDecorations.js';
import { SelectionsOverlay } from '../viewParts/selections/selections.js';
import { ViewCursors } from '../viewParts/viewCursors/viewCursors.js';
import { WhitespaceOverlay, type WhitespaceRenderingMode } from '../viewParts/whitespace/whitespace.js';
import { DynamicViewOverlay } from './dynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from './viewPart.js';

export interface ViewOverlaysOptions {
	readonly contentElement: HTMLDivElement;
	readonly model: TextModel;
	readonly selectionController: CursorsController | undefined;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
	readonly bracketColorizationSource: BracketColorizationSource | undefined;
	readonly decorationSources: readonly DecorationSource[];
	readonly guides: InternalGuidesOptions;
	readonly indentationTabSize: number;
	readonly renderWhitespace: WhitespaceRenderingMode;
	readonly cursorStyle: TextEditorCursorStyle;
	readonly cursorBlinking: TextEditorCursorBlinkingStyle;
	readonly cursorSmoothCaretAnimation: 'off' | 'explicit' | 'on';
	readonly cursorWidth: number;
	readonly cursorHeight: number;
	readonly fontInfo: BareFontInfo;
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
	private readonly indentGuides: IndentGuidesOverlay;
	private readonly viewCursors: ViewCursors;
	private readonly whitespace: WhitespaceOverlay;

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
		this.indentGuides = this.register(new IndentGuidesOverlay(context, {
			host: options.contentElement,
			guides: options.guides,
			tabSize: options.indentationTabSize,
			bracketColorizationSource: options.bracketColorizationSource,
			selectionController: options.selectionController,
		}));
		this.whitespace = this.register(new WhitespaceOverlay(context, options.contentElement, options.model, options.selectionController, options.renderWhitespace));
		this.currentLineHighlight = this.register(new CurrentLineHighlightOverlay(context, options.contentElement, options.selectionController));
		this.selections = this.register(new SelectionsOverlay(context, options.contentElement, options.selectionController));
		this.viewCursors = this.register(new ViewCursors(context, {
			host: options.contentElement,
			style: options.cursorStyle,
			blinking: options.cursorBlinking,
			smoothCaretAnimation: options.cursorSmoothCaretAnimation,
			semanticTokenSource: options.semanticTokenSource,
			lineWidth: options.cursorWidth,
			lineHeight: options.cursorHeight,
			fontInfo: options.fontInfo,
		}, options.model, options.selectionController));
		this.domNodes = Object.freeze([
			this.indentGuides.domNode,
			this.whitespace.domNode,
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

	renderSelection(context: EditorRenderingContext, reason: CursorChangeReason): void {
		this.indentGuides.renderNow(context);
		this.whitespace.renderNow(context);
		this.currentLineHighlight.renderNow(context);
		this.selections.renderNow(context);
		this.viewCursors.renderSelection(context, reason);
	}

	renderCursorTokens(context: EditorRenderingContext): void {
		this.viewCursors.renderTokens(context);
	}

	setCompositionRange(range: Range | undefined): void {
		this.viewCursors.setCompositionRange(range);
	}

	setCursorStyle(style: TextEditorCursorStyle): void {
		this.viewCursors.setStyle(style);
	}

	setCursorLineWidth(lineWidth: number): void {
		this.viewCursors.setLineWidth(lineWidth);
	}

	private register<TPart extends DynamicViewOverlay>(part: TPart): TPart {
		this.parts.push(part);
		this._register(part);
		return part;
	}
}
