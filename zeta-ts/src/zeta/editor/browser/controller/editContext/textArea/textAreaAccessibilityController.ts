import { DisposableOwner } from '../../../../../base/common/lifecycle.js';
import { type EditorSelectionController } from '../../../../common/cursor/editorSelectionController.js';
import { TextSelection, TextSelectionSet } from '../../../../common/core/selection.js';
import { TextRange } from '../../../../common/core/text.js';
import { type EditorViewport } from '../../../../browser/view.js';
import { type TextModel } from '../../../../common/model/textModel.js';
import { type CompositionController } from '../../compositionController.js';
import { modelOffsetAtContentOffset, SimplePagedScreenReaderStrategy, type ISimpleScreenReaderContentState } from '../screenReaderUtils.js';
import { type TextAreaEditContext } from './textAreaEditContext.js';
import { TextAreaState } from './textAreaEditContextState.js';

const ACCESSIBILITY_LINES_PER_PAGE = 500;
const MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS = 32 * 1_024;

/** Mirrors the active editor window into the native input for assistive technology. */
export class TextAreaAccessibilityController extends DisposableOwner {
	private accessibleInputSyncScheduled = false;
	private accessibleInputState = TextAreaState.EMPTY;
	private accessibleScreenReaderContentState: ISimpleScreenReaderContentState | undefined;
	private accessibleInputStartOffset = 0;
	private readonly screenReaderStrategy = new SimplePagedScreenReaderStrategy();

	constructor(
		private readonly input: TextAreaEditContext,
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
		private readonly compositionController: CompositionController,
	) {
		super();
		this.own(input.onDidFocus(() => this.synchronizeAccessibleInput()));
		this.own(input.onDidBlur(() => {
			this.accessibleInputState = TextAreaState.EMPTY;
			this.accessibleScreenReaderContentState = undefined;
			this.accessibleInputStartOffset = 0;
		}));
		this.own(input.onDidSelect(() => this.acceptAccessibleSelection()));
		this.own(viewport.textModel.onDidChange(() => this.scheduleAccessibleInputSynchronization()));
		this.own(selectionController.onDidChange(() => this.scheduleAccessibleInputSynchronization()));
		this.own(compositionController.onDidChange(() => this.scheduleAccessibleInputSynchronization()));
	}

	synchronizeAccessibleInput(): void {
		if (this.isDisposed || this.compositionController.composing || this.input.element.ownerDocument.activeElement !== this.input.element) return;
		const model = this.viewport.textModel;
		const selection = this.selectionController.selections.primary;
		this.updateAccessibleSelectionDescription();
		if (model.length > MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS) {
			const selectionStartOffset = model.offsetAt(selection.range.start);
			const selectionEndOffset = model.offsetAt(selection.range.end);
			const window = accessibleInputWindow(
				model.length,
				selectionStartOffset,
				selectionEndOffset,
				model.offsetAt(selection.active),
			);
			const text = model.getTextInRange(TextRange.from(model.positionAt(window.startOffset), model.positionAt(window.endOffset)));
			this.accessibleInputStartOffset = window.startOffset;
			this.accessibleScreenReaderContentState = undefined;
			this.accessibleInputState = new TextAreaState(
				text,
				selection.direction === 'backward'
					? clampOffset(selectionEndOffset - window.startOffset, text.length)
					: clampOffset(selectionStartOffset - window.startOffset, text.length),
				selection.direction === 'backward'
					? clampOffset(selectionStartOffset - window.startOffset, text.length)
					: clampOffset(selectionEndOffset - window.startOffset, text.length),
				selection.range,
				undefined,
			);
			this.accessibleInputState.writeToTextArea('screenReaderContent', this.input, true);
			return;
		}
		const contentState = this.screenReaderStrategy.fromEditorSelection(
			model,
			selection,
			ACCESSIBILITY_LINES_PER_PAGE,
			true,
		);
		const state = TextAreaState.fromScreenReaderContentState(contentState);
		this.accessibleScreenReaderContentState = contentState;
		this.accessibleInputState = state;
		this.accessibleInputStartOffset = contentState.startOffset;
		state.writeToTextArea('screenReaderContent', this.input, true);
	}

	private updateAccessibleSelectionDescription(): void {
		const selections = this.selectionController.selections;
		if (selections.selections.length === 1) {
			this.input.element.removeAttribute('aria-description');
			return;
		}
		const primary = selections.primary.active;
		this.input.element.setAttribute(
			'aria-description',
			`${selections.selections.length} selections. Primary at line ${primary.lineIndex + 1}, column ${primary.columnIndex + 1}.`,
		);
	}

	private scheduleAccessibleInputSynchronization(): void {
		if (this.accessibleInputSyncScheduled) return;
		this.accessibleInputSyncScheduled = true;
		queueMicrotask(() => {
			this.accessibleInputSyncScheduled = false;
			this.synchronizeAccessibleInput();
		});
	}

	private acceptAccessibleSelection(): void {
		if (this.compositionController.composing || this.input.element.ownerDocument.activeElement !== this.input.element) return;
		const model = this.viewport.textModel;
		this.accessibleInputState = TextAreaState.readFromTextArea(this.input, this.accessibleInputState);
		const startOffset = this.input.element.selectionStart;
		const endOffset = this.input.element.selectionEnd;
		const backward = this.input.element.selectionDirection === 'backward';
		const contentState = this.accessibleScreenReaderContentState;
		if (!contentState) {
			const anchorOffset = this.accessibleInputStartOffset + (backward ? endOffset : startOffset);
			const activeOffset = this.accessibleInputStartOffset + (backward ? startOffset : endOffset);
			this.applyAccessibleSelection(model, anchorOffset, activeOffset);
			return;
		}
		const anchorOffset = modelOffsetAtContentOffset(
			contentState,
			backward ? endOffset : startOffset,
			backward ? 'end' : 'start',
		);
		const activeOffset = modelOffsetAtContentOffset(
			contentState,
			backward ? startOffset : endOffset,
			backward ? 'start' : 'end',
		);
		this.applyAccessibleSelection(model, anchorOffset, activeOffset);
	}

	private applyAccessibleSelection(model: TextModel, anchorOffset: number, activeOffset: number): void {
		const safeAnchorOffset = clampOffset(anchorOffset, model.length);
		const safeActiveOffset = clampOffset(activeOffset, model.length);
		const current = this.selectionController.selections.primary;
		if (model.offsetAt(current.anchor) === safeAnchorOffset && model.offsetAt(current.active) === safeActiveOffset) return;
		this.selectionController.setSelections(TextSelectionSet.single(TextSelection.from(
			model.positionAt(safeAnchorOffset),
			model.positionAt(safeActiveOffset),
		)));
		this.viewport.revealPosition(this.selectionController.selections.primary.active);
	}
}

function accessibleInputWindow(modelLength: number, selectionStartOffset: number, selectionEndOffset: number, activeOffset: number): { readonly startOffset: number; readonly endOffset: number } {
	if (modelLength <= MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS) return { startOffset: 0, endOffset: modelLength };
	const selectionLength = selectionEndOffset - selectionStartOffset;
	if (selectionLength <= MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS) {
		const margin = Math.floor((MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS - selectionLength) / 2);
		let startOffset = Math.max(0, selectionStartOffset - margin);
		startOffset = Math.min(startOffset, modelLength - MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS);
		if (selectionEndOffset > startOffset + MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS) startOffset = selectionEndOffset - MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS;
		return { startOffset, endOffset: startOffset + MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS };
	}
	const startOffset = Math.min(Math.max(0, activeOffset - Math.floor(MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS / 2)), modelLength - MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS);
	return { startOffset, endOffset: startOffset + MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS };
}

function clampOffset(offset: number, length: number): number {
	return Math.min(Math.max(0, offset), length);
}
