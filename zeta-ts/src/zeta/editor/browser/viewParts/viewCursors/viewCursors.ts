import './viewCursors.css';
import { h, reset } from '../../../../base/browser/dom.js';
import { FastDomNode, createFastDomNode } from '../../../../base/browser/fastDomNode.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { TextEditorCursorBlinkingStyle, TextEditorCursorStyle } from '../../../common/config/editorOptions.js';
import { CursorChangeReason } from '../../../common/cursorEvents.js';
import { type Range } from '../../../common/core/range.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type IViewModel } from '../../../common/viewModel.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import { type TrackedRange } from '../../../common/model/trackedRange.js';
import { type SemanticTokenSource } from '../../../common/services/resolvedSemanticTokens.js';
import { createStanzaVisualRangeRectangles } from '../../../common/viewModel/visualRangeGeometry.js';
import { type RenderingContext } from '../../view/renderingContext.js';
import { ViewPart } from '../../view/viewPart.js';
import { ViewPartRows } from '../../view/viewLayer.js';
import { CursorPlurality, ViewCursor, type IViewCursorRenderData, type ViewCursorOptions } from './viewCursor.js';
import { TrackedRangeStickiness } from '../../../common/model.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type TextMeasurer } from '../../../common/viewModel/textMeasurer.js';

export interface ViewCursorsOptions extends ViewCursorOptions {
	readonly host: HTMLElement;
	readonly blinking: TextEditorCursorBlinkingStyle;
	readonly smoothCaretAnimation: 'off' | 'explicit' | 'on';
	readonly semanticTokenSource?: SemanticTokenSource;
}

/** Coordinates active cursors, movement animation, and input composition presentation. */
export class ViewCursors extends ViewPart {
	public readonly domNode: HTMLElement;
	private readonly fastDomNode: FastDomNode<HTMLElement>;
	private readonly model: TextModel;
	private readonly viewModel: IViewModel;
	private readonly semanticTokenSource: SemanticTokenSource | undefined;
	private readonly compositionRows: ViewPartRows;
	private readonly smoothCaretAnimation: 'off' | 'explicit' | 'on';
	private cursorOptions: ViewCursorOptions;
	private readonly cursors: ViewCursor[] = [];
	private compositionRange: TrackedRange | undefined;
	private previousSelectionCount: number;
	private pauseMovementAnimation = true;
	private movementRenderGeneration = 0;
	private renderData: IViewCursorRenderData[] = [];

	constructor(context: ViewContext, options: ViewCursorsOptions, model: TextModel, viewModel: IViewModel) {
		super(context);
		this.domNode = h(options.host.ownerDocument, 'div');
		this.fastDomNode = createFastDomNode(this.domNode);
		this.fastDomNode.setClassName('cursors-layer stanza-editor-cursors-layer');
		this.domNode.classList.add(cursorBlinkingClass(options.blinking));
		this.domNode.classList.toggle('cursor-smooth-caret-animation', options.smoothCaretAnimation !== 'off');
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
		this._register(toDisposable(() => this.domNode.remove()));
		this.compositionRows = this._register(new ViewPartRows(this.domNode, 'stanza-editor-composition-layer', 'stanza-editor-composition-row'));
		this.domNode.append(this.compositionRows.domNode.domNode);
		this.model = model;
		this.viewModel = viewModel;
		this.semanticTokenSource = options.semanticTokenSource;
		this.smoothCaretAnimation = options.smoothCaretAnimation;
		this.cursorOptions = options;
		this.previousSelectionCount = viewModel.getCursorStates().length;
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
	}

	public getDomNode(): FastDomNode<HTMLElement> {
		return this.fastDomNode;
	}

	public setStyle(style: TextEditorCursorStyle): void {
		if (style === this.cursorOptions.style) return;
		this.cursorOptions = Object.freeze({ ...this.cursorOptions, style });
		for (const cursor of this.cursors) cursor.onConfigurationChanged(this.cursorOptions);
	}

	public setLineWidth(lineWidth: number): void {
		if (lineWidth === this.cursorOptions.lineWidth) return;
		this.cursorOptions = Object.freeze({ ...this.cursorOptions, lineWidth });
		for (const cursor of this.cursors) cursor.onConfigurationChanged(this.cursorOptions);
	}

	public override prepareRender(context: RenderingContext): void {
		if (this.cursors.length !== this.viewModel.getCursorStates().length) this.reconcileCursors(this.pauseMovementAnimation);
		else this.updateCursorPositions(this.pauseMovementAnimation);
		for (const cursor of this.cursors) cursor.prepareRender(context);
	}

	public render(context: RenderingContext): void {
		const rows = this.compositionRows.render(context);
		for (const row of rows.values()) reset(row);
		projectStanzaCompositionOverlay(context, this.model, this.cursorOptions.readVisualProjection(), this.cursorOptions.readTextLeft(), this.cursorOptions.textMeasurer, this.compositionRange?.range, rows);
		this.renderData = [];
		for (const cursor of this.cursors) {
			const renderData = cursor.render(context);
			if (renderData) this.renderData.push(renderData);
		}
	}

	public getLastRenderData(): IViewCursorRenderData[] {
		return this.renderData;
	}

	public renderSelection(context: RenderingContext, reason: CursorChangeReason): void {
		const selectionCount = this.viewModel.getCursorStates().length;
		this.pauseMovementAnimation = !this.shouldAnimateMovement(reason, selectionCount);
		this.previousSelectionCount = selectionCount;
		this.reconcileCursors(this.pauseMovementAnimation);
		this.prepareRender(context);
		this.render(context);
		for (const animation of this.domNode.getAnimations?.() ?? []) animation.currentTime = 0;
		const generation = ++this.movementRenderGeneration;
		queueMicrotask(() => {
			if (generation === this.movementRenderGeneration) this.pauseMovementAnimation = true;
		});
	}

	public renderTokens(context: RenderingContext): void {
		this.movementRenderGeneration += 1;
		this.pauseMovementAnimation = true;
		this.prepareRender(context);
		this.render(context);
	}

	private reconcileCursors(pauseMovementAnimation: boolean): void {
		const selections = this.viewModel.getCursorStates().map(state => state.modelState.selection);
		const selectionCount = selections.length;
		while (this.cursors.length < selectionCount) {
			const selectionIndex = this.cursors.length;
			this.cursors.push(new ViewCursor(
				this._context,
				this.domNode,
				selectionIndex,
				this.cursorOptions,
				this.model,
				this.semanticTokenSource,
				cursorPlurality(selectionIndex, selectionCount, 0),
			));
		}
		while (this.cursors.length > selectionCount) this.cursors.pop()!.dispose();
		this.updateCursorPositions(pauseMovementAnimation);
	}

	private updateCursorPositions(pauseMovementAnimation: boolean): void {
		const selections = this.viewModel.getCursorStates().map(state => state.modelState.selection);
		for (let selectionIndex = 0; selectionIndex < this.cursors.length; selectionIndex += 1) {
			const plurality = cursorPlurality(selectionIndex, this.cursors.length, 0);
			const cursor = this.cursors[selectionIndex]!;
			cursor.setPlurality(plurality);
			cursor.onCursorPositionChanged(selections[selectionIndex]!.getPosition(), pauseMovementAnimation);
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

function projectStanzaCompositionOverlay(context: RenderingContext, model: TextModel, projection: EditorVisualLineProjection, textLeft: number, textMeasurer: TextMeasurer, range: Range | undefined, rows: ReadonlyMap<number, HTMLElement>): void {
	if (!range) return;
	const visibleRanges = context.linesVisibleRangesForRange(range, false);
	if (visibleRanges) {
		for (const line of visibleRanges) {
			for (const horizontalRange of line.ranges) appendCompositionRange(rows, line.lineNumber - 1, horizontalRange.left, horizontalRange.width);
		}
		return;
	}
	const renderLines = { startLineIndex: context.viewportData.startLineNumber - 1, endLineIndexExclusive: context.viewportData.endLineNumber };
	const rectangles = createStanzaVisualRangeRectangles(model, [{ range, value: undefined }], projection, renderLines, textLeft, textMeasurer);
	for (const rectangle of rectangles) appendCompositionRange(rows, rectangle.visualLineIndex, rectangle.left, rectangle.width);
}


function appendCompositionRange(rows: ReadonlyMap<number, HTMLElement>, visualLineIndex: number, left: number, width: number): void {
	const row = rows.get(visualLineIndex);
	if (!row) return;
	const element = h(row.ownerDocument, 'div');
	element.className = 'stanza-editor-composition';
	element.style.left = `${left}px`;
	element.style.width = `${width}px`;
	row.append(element);
}
