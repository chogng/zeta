import './textAreaEditContext.css';
import { Disposable, toDisposable } from "../../../../../base/common/lifecycle.js";
import { h } from "../../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../../base/browser/fastDomNode.js";
import { type Event } from "../../../../../base/common/event.js";
import { type CompositionController, type EditContextCompositionEvent, EditContext, type EditContextOptions, type EditContextPosition, type EditContextState } from "../editContext.js";
import { type CursorsController } from '../../../../common/cursor/cursor.js';
import { TextSelection, TextSelectionSet } from '../../../../common/core/selection.js';
import { TextRange } from '../../../../common/core/text.js';
import { type TextModel } from '../../../../common/model/textModel.js';
import { type EditorViewport } from '../../../view.js';
import { modelOffsetAtContentOffset, SimplePagedScreenReaderStrategy, type ISimpleScreenReaderContentState } from '../screenReaderUtils.js';
import { TextAreaInput } from "./textAreaEditContextInput.js";
import { TextAreaEditContextRegistry } from "./textAreaEditContextRegistry.js";
import { TextAreaState, type ITextAreaWrapper } from "./textAreaEditContextState.js";

/** Options accepted by the textarea-backed edit context. */
export type TextAreaEditContextOptions = EditContextOptions;

/**
 * Textarea fallback for browsers without the native EditContext API.
 *
 * The textarea remains deliberately ignorant of editor state. Accessibility
 * mirroring is layered on by TextAreaAccessibilityController, while this
 * class only translates browser events and owns the transient IME element.
 */
export class TextAreaEditContext extends EditContext implements ITextAreaWrapper {
	readonly inputNode: FastDomNode<HTMLTextAreaElement>;
	readonly element: HTMLTextAreaElement;
	readonly textArea: HTMLTextAreaElement;
	readonly textAreaInput: TextAreaInput;
	private connected = false;

	get onDidFocus(): Event<void> { return this.textAreaInput.onDidFocus; }
	get onDidBlur(): Event<void> { return this.textAreaInput.onDidBlur; }
	get onDidBeforeInput(): Event<InputEvent> { return this.textAreaInput.onDidBeforeInput; }
	get onDidInput(): Event<InputEvent> { return this.textAreaInput.onDidInput; }
	get onDidSelect(): Event<void> { return this.textAreaInput.onDidSelect; }
	get onDidKeydown(): Event<KeyboardEvent> { return this.textAreaInput.onDidKeydown; }
	get onDidCompositionStart(): Event<EditContextCompositionEvent> { return this.textAreaInput.onDidCompositionStart; }
	get onDidCompositionUpdate(): Event<EditContextCompositionEvent> { return this.textAreaInput.onDidCompositionUpdate; }
	get onDidCompositionEnd(): Event<EditContextCompositionEvent> { return this.textAreaInput.onDidCompositionEnd; }

	constructor(
		private readonly container: HTMLElement,
		options: TextAreaEditContextOptions = {},
	) {
		super();
		const ownerDocument = container.ownerDocument;
		this.inputNode = new FastDomNode(h(ownerDocument, "textarea"));
		this.element = this.inputNode.domNode;
		this.textArea = this.element;
		this.inputNode.setClassName("stanza-editor-input");
		this.inputNode.domNode.tabIndex = -1;
		this.element.spellcheck = false;
		this.element.readOnly = options.readOnly ?? false;
		this.element.wrap = "off";
		this.element.dir = options.textDirection ?? "auto";
		this.element.autocomplete = "off";
		this.element.setAttribute("autocapitalize", "off");
		this.element.setAttribute("aria-label", options.ariaLabel ?? "Stanza editor input");
		this.element.setAttribute("aria-multiline", "true");
		this.element.setAttribute("aria-roledescription", "code editor");
		this.element.setAttribute("aria-readonly", String(this.element.readOnly));
		this.textAreaInput = this._register(new TextAreaInput(this.element));
		if (options.ownerId !== undefined) this._register(TextAreaEditContextRegistry.register(options.ownerId, this));
		this._register(TextAreaEditContextRegistry.register(this.element, this));
		container.append(this.element);
		this._register(toDisposable(() => this.element.remove()));
	}

	get readOnly(): boolean {
		return this.element.readOnly;
	}

	/**
	 * Installs DOM listeners after higher-level consumers have subscribed to the
	 * edit-context events. This preserves completion and clipboard ordering.
	 */
	connect(): void {
		this.assertNotDisposed();
		if (this.connected) return;
		this.connected = true;
		this._register(this.textAreaInput.onDidCopy(event => this.fireWillCopy(event, false)));
		this._register(this.textAreaInput.onDidCut(event => this.fireWillCopy(event, true)));
		this._register(this.textAreaInput.onDidPaste(event => this.fireWillPaste(event)));
		this.textAreaInput.connect();
	}

	focus(): void {
		this.textAreaInput.focus();
	}

	clear(): void {
		this.textAreaInput.clear();
	}

	getValue(): string {
		return this.textAreaInput.getValue();
	}

	setValue(reason: string, value: string): void {
		this.textAreaInput.setValue(reason, value);
	}

	getSelectionStart(): number {
		return this.textAreaInput.getSelectionStart();
	}

	getSelectionEnd(): number {
		return this.textAreaInput.getSelectionEnd();
	}

	setSelectionRange(reason: string, selectionStart: number, selectionEnd: number): void {
		this.textAreaInput.setSelectionRange(reason, selectionStart, selectionEnd);
	}

	/** The accessibility controller is the state mirror for textarea input. */
	syncState(_state: EditContextState): void {}

	/** Textarea accessibility geometry is maintained by its dedicated controller. */
	updateBounds(_position: EditContextPosition): void {}

	setReadOnly(readOnly: boolean): void {
		this.element.readOnly = readOnly;
		this.element.setAttribute("aria-readonly", String(readOnly));
	}

	prepareComposition(): void {
		this.textAreaInput.clear();
		this.inputNode.toggleClassName("ime-input", true);
	}

	positionComposition(position: EditContextPosition): void {
		this.inputNode.setLeft(position.left);
		this.inputNode.setTop(position.top);
		this.inputNode.setHeight(position.height);
	}

	clearComposition(): void {
		this.textAreaInput.clear();
		this.inputNode.toggleClassName("ime-input", false);
		this.inputNode.setLeft("");
		this.inputNode.setTop("");
		this.inputNode.setHeight("");
	}
}

const ACCESSIBILITY_LINES_PER_PAGE = 500;
const MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS = 32 * 1_024;

/** Mirrors the active editor window into the native input for assistive technology. */
export class TextAreaAccessibilityController extends Disposable {
	private accessibleInputSyncScheduled = false;
	private accessibleInputState = TextAreaState.EMPTY;
	private accessibleScreenReaderContentState: ISimpleScreenReaderContentState | undefined;
	private accessibleInputStartOffset = 0;
	private readonly screenReaderStrategy = new SimplePagedScreenReaderStrategy();

	constructor(
		private readonly input: TextAreaEditContext,
		private readonly viewport: EditorViewport,
		private readonly selectionController: CursorsController,
		private readonly compositionController: CompositionController,
	) {
		super();
		this._register(input.onDidFocus(() => this.synchronizeAccessibleInput()));
		this._register(input.onDidBlur(() => {
			this.accessibleInputState = TextAreaState.EMPTY;
			this.accessibleScreenReaderContentState = undefined;
			this.accessibleInputStartOffset = 0;
		}));
		this._register(input.onDidSelect(() => this.acceptAccessibleSelection()));
		this._register(viewport.textModel.onDidChange(() => this.scheduleAccessibleInputSynchronization()));
		this._register(selectionController.onDidChange(() => this.scheduleAccessibleInputSynchronization()));
		this._register(compositionController.onDidChange(() => this.scheduleAccessibleInputSynchronization()));
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
