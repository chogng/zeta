import './viewCursors.css';
import { h, reset } from '../../../../base/browser/dom.js';
import { FastDomNode, createFastDomNode } from '../../../../base/browser/fastDomNode.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { TextEditorCursorBlinkingStyle, TextEditorCursorStyle } from '../../../common/config/editorOptions.js';
import { type CursorsController } from '../../../common/cursor/cursor.js';
import { CursorChangeReason } from '../../../common/cursorEvents.js';
import { type Range } from '../../../common/core/range.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type TrackedRange } from '../../../common/model/trackedRange.js';
import { type SemanticTokenSource } from '../../../common/services/resolvedSemanticTokens.js';
import { createStanzaVisualRangeRectangles } from '../../../common/viewModel/visualRangeGeometry.js';
import { type EditorOverlayContext } from '../../view/renderingContext.js';
import { EditorDynamicViewOverlay } from '../../view/editorDynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from '../../view/viewPart.js';
import { ViewPartRows } from '../../view/viewLayer.js';
import { CursorPlurality, EditorViewCursor, type IViewCursorRenderData, type ViewCursorOptions } from './viewCursor.js';
import { TrackedRangeStickiness } from '../../../common/model.js';

export interface ViewCursorsOptions extends ViewCursorOptions {
	readonly host: HTMLElement;
	readonly blinking: TextEditorCursorBlinkingStyle;
	readonly smoothCaretAnimation: 'off' | 'explicit' | 'on';
	readonly semanticTokenSource?: SemanticTokenSource;
}

/** Coordinates active cursors, movement animation, and input composition presentation. */
export class EditorViewCursors extends EditorDynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly fastDomNode: FastDomNode<HTMLElement>;
	private readonly model: TextModel;
	private readonly selectionController: CursorsController | undefined;
	private readonly semanticTokenSource: SemanticTokenSource | undefined;
	private readonly compositionRows: ViewPartRows;
	private readonly smoothCaretAnimation: 'off' | 'explicit' | 'on';
	private cursorOptions: ViewCursorOptions;
	private readonly cursors: EditorViewCursor[] = [];
	private compositionRange: TrackedRange | undefined;
	private previousSelectionCount: number;
	private pauseMovementAnimation = true;
	private movementRenderGeneration = 0;
	private renderData: IViewCursorRenderData[] = [];

	constructor(context: EditorViewContext, options: ViewCursorsOptions, model: TextModel, selectionController: CursorsController | undefined) {
		super(context);
		this.domNode = h(options.host.ownerDocument, 'div');
		this.fastDomNode = createFastDomNode(this.domNode);
		this.fastDomNode.setClassName('stanza-editor-cursors-layer');
		this.domNode.classList.add(cursorBlinkingClass(options.blinking));
		this.domNode.classList.toggle('cursor-smooth-caret-animation', options.smoothCaretAnimation !== 'off');
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
		this._register(toDisposable(() => this.domNode.remove()));
		this.compositionRows = this._register(new ViewPartRows(this.domNode, 'stanza-editor-composition-layer', 'stanza-editor-composition-row'));
		this.domNode.append(this.compositionRows.domNode);
		this.model = model;
		this.selectionController = selectionController;
		this.semanticTokenSource = options.semanticTokenSource;
		this.smoothCaretAnimation = options.smoothCaretAnimation;
		this.cursorOptions = options;
		this.previousSelectionCount = selectionController?.selections.selections.length ?? 0;
		this.reconcileCursors(true);
		this._register(toDisposable(() => {
			for (const cursor of this.cursors.splice(0)) cursor.dispose();
		}));
		this._register(toDisposable(() => this.compositionRange?.dispose()));
	}

	public setCompositionRange(range: Range | undefined): void {
		const next = range ? this.model.trackRange(range, TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges) : undefined;
		this.compositionRange?.dispose();
		this.compositionRange = next;
		this.renderNow(this.context.renderingContext);
	}

	public getDomNode(): FastDomNode<HTMLElement> {
		return this.fastDomNode;
	}

	public setStyle(style: TextEditorCursorStyle): void {
		if (style === this.cursorOptions.style) return;
		this.cursorOptions = Object.freeze({ ...this.cursorOptions, style });
		for (const cursor of this.cursors) cursor.onConfigurationChanged(this.cursorOptions);
		this.renderNow(this.context.renderingContext);
	}

	public setLineWidth(lineWidth: number): void {
		if (lineWidth === this.cursorOptions.lineWidth) return;
		this.cursorOptions = Object.freeze({ ...this.cursorOptions, lineWidth });
		for (const cursor of this.cursors) cursor.onConfigurationChanged(this.cursorOptions);
		this.renderNow(this.context.renderingContext);
	}

	public override prepareRender(context: EditorRenderingContext): void {
		this.updateCursorPositions(this.pauseMovementAnimation);
		for (const cursor of this.cursors) cursor.prepareRender(context);
	}

	public render(context: EditorRenderingContext): void {
		const rows = this.compositionRows.render(context);
		for (const row of rows.values()) reset(row);
		if (context.overlay) projectStanzaCompositionOverlay(context.overlay, this.compositionRange?.range, rows);
		this.renderData = [];
		for (const cursor of this.cursors) {
			const renderData = cursor.render(context);
			if (renderData) this.renderData.push(renderData);
		}
	}

	public getLastRenderData(): IViewCursorRenderData[] {
		return this.renderData;
	}

	public renderSelection(context: EditorRenderingContext, reason: CursorChangeReason): void {
		const selectionCount = this.selectionController?.selections.selections.length ?? 0;
		this.pauseMovementAnimation = !this.shouldAnimateMovement(reason, selectionCount);
		this.previousSelectionCount = selectionCount;
		this.reconcileCursors(this.pauseMovementAnimation);
		this.renderNow(context);
		for (const animation of this.domNode.getAnimations?.() ?? []) animation.currentTime = 0;
		const generation = ++this.movementRenderGeneration;
		queueMicrotask(() => {
			if (generation === this.movementRenderGeneration) this.pauseMovementAnimation = true;
		});
	}

	public renderTokens(context: EditorRenderingContext): void {
		this.movementRenderGeneration += 1;
		this.pauseMovementAnimation = true;
		this.renderNow(context);
	}

	private reconcileCursors(pauseMovementAnimation: boolean): void {
		const selections = this.selectionController?.selections;
		const selectionCount = selections?.selections.length ?? 0;
		while (this.cursors.length < selectionCount) {
			const selectionIndex = this.cursors.length;
			this.cursors.push(new EditorViewCursor(
				this.context,
				this.domNode,
				selectionIndex,
				this.cursorOptions,
				this.model,
				this.semanticTokenSource,
				cursorPlurality(selectionIndex, selectionCount, selections?.primaryIndex ?? 0),
			));
		}
		while (this.cursors.length > selectionCount) this.cursors.pop()!.dispose();
		this.updateCursorPositions(pauseMovementAnimation);
	}

	private updateCursorPositions(pauseMovementAnimation: boolean): void {
		const selections = this.selectionController?.selections;
		if (!selections) return;
		for (let selectionIndex = 0; selectionIndex < this.cursors.length; selectionIndex += 1) {
			const plurality = cursorPlurality(selectionIndex, this.cursors.length, selections.primaryIndex);
			const cursor = this.cursors[selectionIndex]!;
			cursor.setPlurality(plurality);
			cursor.onCursorPositionChanged(selections.selections[selectionIndex]!.getPosition(), pauseMovementAnimation);
		}
	}

	private shouldAnimateMovement(reason: CursorChangeReason, selectionCount: number): boolean {
		if (this.smoothCaretAnimation === 'off' || selectionCount !== this.previousSelectionCount) return false;
		if (this.smoothCaretAnimation === 'on') return true;
		return reason === CursorChangeReason.Explicit;
	}
}

function cursorPlurality(selectionIndex: number, selectionCount: number, primaryIndex: number): CursorPlurality {
	if (selectionCount === 1) return CursorPlurality.Single;
	return selectionIndex === primaryIndex ? CursorPlurality.MultiPrimary : CursorPlurality.MultiSecondary;
}

function cursorBlinkingClass(blinking: TextEditorCursorBlinkingStyle): string {
	switch (blinking) {
		case TextEditorCursorBlinkingStyle.Smooth: return 'cursor-blinking-smooth';
		case TextEditorCursorBlinkingStyle.Phase: return 'cursor-blinking-phase';
		case TextEditorCursorBlinkingStyle.Expand: return 'cursor-blinking-expand';
		case TextEditorCursorBlinkingStyle.Solid: return 'cursor-blinking-solid';
		case TextEditorCursorBlinkingStyle.Hidden: return 'cursor-blinking-hidden';
		default: return 'cursor-blinking-blink';
	}
}

function projectStanzaCompositionOverlay(context: EditorOverlayContext, range: Range | undefined, rows: ReadonlyMap<number, HTMLElement>): void {
	if (!range) return;
	const rectangles = context.linesVisibleRangesForRange(range, false)
		?? createStanzaVisualRangeRectangles(context.model, [{ range, value: undefined }], context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of rectangles) {
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		const element = h(context.ownerDocument, 'div');
		element.className = 'stanza-editor-composition';
		element.style.left = `${rectangle.left}px`;
		element.style.width = `${rectangle.width}px`;
		row.append(element);
	}
}
