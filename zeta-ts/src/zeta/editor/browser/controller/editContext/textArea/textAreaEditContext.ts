import './textAreaEditContext.css';
import { Disposable, toDisposable } from "../../../../../base/common/lifecycle.js";
import { h } from "../../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../../base/browser/fastDomNode.js";
import { type Event } from "../../../../../base/common/event.js";
import { AbstractEditContext, type CompositionController, type EditContextCompositionEvent, type EditContextOptions, type EditContextPosition, type EditContextState } from "../editContext.js";
import { type IEditorAriaOptions } from '../../../editorBrowser.js';
import { type CursorsController } from '../../../../common/cursor/cursor.js';
import { SelectionDirection, Selection } from '../../../../common/core/selection.js';
import { SelectionSet } from '../../../../common/cursor/selectionSet.js';
import { Position } from '../../../../common/core/position.js';
import { Range } from '../../../../common/core/range.js';
import { type TextModel } from '../../../../common/model/textModel.js';
import { type EditorViewport } from '../../../view.js';
import { modelOffsetAtContentOffset, SimplePagedScreenReaderStrategy, type ISimpleScreenReaderContentState } from '../screenReaderUtils.js';
import { TextAreaInput } from "./textAreaEditContextInput.js";
import { TextAreaEditContextRegistry } from "./textAreaEditContextRegistry.js";
import { TextAreaState, type ITextAreaWrapper } from "./textAreaEditContextState.js";

/** Options accepted by the textarea-backed edit context. */
export type TextAreaEditContextOptions = EditContextOptions;

/**
 * Textarea implementation used when the browser has no EditContext API.
 *
 * The concrete edit context owns its textarea input, composition controller,
 * focus state, ARIA state, and screen-reader mirror.
 */
export class TextAreaEditContext extends AbstractEditContext implements ITextAreaWrapper {
	readonly inputNode: FastDomNode<HTMLTextAreaElement>;
	readonly domNode: HTMLTextAreaElement;
	readonly textArea: HTMLTextAreaElement;
	readonly textAreaInput: TextAreaInput;
	private readonly accessibilityController: TextAreaAccessibilityController;
	private lastRenderPosition: Position | null = null;
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
		options: TextAreaEditContextOptions,
	) {
		super();
		const ownerDocument = container.ownerDocument;
		this.inputNode = new FastDomNode(h(ownerDocument, "textarea"));
		this.domNode = this.inputNode.domNode;
		this.textArea = this.domNode;
		this.inputNode.setClassName("stanza-editor-input");
		this.inputNode.domNode.tabIndex = -1;
		this.domNode.spellcheck = false;
		this.domNode.readOnly = options.readOnly;
		this.domNode.wrap = "off";
		this.domNode.dir = options.textDirection;
		this.domNode.autocomplete = "off";
		this.domNode.setAttribute("autocapitalize", "off");
		this.domNode.setAttribute("aria-label", options.ariaLabel ?? "Stanza editor input");
		this.domNode.setAttribute("aria-multiline", "true");
		this.domNode.setAttribute("aria-roledescription", "code editor");
		this.domNode.setAttribute("aria-readonly", String(this.domNode.readOnly));
		this.textAreaInput = this._register(new TextAreaInput(this.domNode));
		this._register(TextAreaEditContextRegistry.register(options.ownerId, this));
		this._register(TextAreaEditContextRegistry.register(this.domNode, this));
		container.append(this.domNode);
		this._register(toDisposable(() => this.domNode.remove()));
		const compositionController = this.initializeController(options);
		this.accessibilityController = this._register(new TextAreaAccessibilityController(this, options.viewport, options.selectionController, compositionController));
	}

	get readOnly(): boolean {
		return this.domNode.readOnly;
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

	isFocused(): boolean {
		return this.textAreaInput.isFocused();
	}

	refreshFocusState(): void {
		this.textAreaInput.refreshFocusState();
	}

	setAriaOptions(options: IEditorAriaOptions): void {
		if (options.activeDescendant) {
			this.domNode.setAttribute('aria-haspopup', 'true');
			this.domNode.setAttribute('aria-autocomplete', 'list');
			this.domNode.setAttribute('aria-activedescendant', options.activeDescendant);
		} else {
			this.domNode.setAttribute('aria-haspopup', 'false');
			this.domNode.setAttribute('aria-autocomplete', 'both');
			this.domNode.removeAttribute('aria-activedescendant');
		}
		if (options.role) this.domNode.setAttribute('role', options.role);
	}

	getLastRenderData(): Position | null {
		return this.lastRenderPosition;
	}

	writeScreenReaderContent(reason: string): void {
		this.accessibilityController.writeScreenReaderContent(reason);
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
	syncState(state: EditContextState): void {
		this.lastRenderPosition = state.position;
	}

	/** Textarea accessibility geometry is maintained by its dedicated controller. */
	updateBounds(_position: EditContextPosition): void {}

	setReadOnly(readOnly: boolean): void {
		this.domNode.readOnly = readOnly;
		this.domNode.setAttribute("aria-readonly", String(readOnly));
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
class TextAreaAccessibilityController extends Disposable {
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
		this._register(input.onDidFocus(() => this.writeScreenReaderContent('focus')));
		this._register(input.onDidBlur(() => {
			this.accessibleInputState = TextAreaState.EMPTY;
			this.accessibleScreenReaderContentState = undefined;
			this.accessibleInputStartOffset = 0;
		}));
		this._register(input.onDidSelect(() => this.acceptAccessibleSelection()));
		this._register(compositionController.onDidChange(() => this.scheduleScreenReaderContent()));
	}

	writeScreenReaderContent(reason: string): void {
		void reason;
		if (this.isDisposed || this.compositionController.composing || this.input.domNode.ownerDocument.activeElement !== this.input.domNode) return;
		const model = this.viewport.textModel;
		const selection = this.selectionController.selections.primary;
		this.updateAccessibleSelectionDescription();
		if (model.length > MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS) {
			const selectionStartOffset = model.offsetAt(selection.getStartPosition());
			const selectionEndOffset = model.offsetAt(selection.getEndPosition());
			const window = accessibleInputWindow(
				model.length,
				selectionStartOffset,
				selectionEndOffset,
				model.offsetAt(selection.getPosition()),
			);
			const text = model.getTextInRange(Range.fromPositions(model.positionAt(window.startOffset), model.positionAt(window.endOffset)));
			this.accessibleInputStartOffset = window.startOffset;
			this.accessibleScreenReaderContentState = undefined;
			this.accessibleInputState = new TextAreaState(
				text,
				selection.getDirection() === SelectionDirection.RTL
					? clampOffset(selectionEndOffset - window.startOffset, text.length)
					: clampOffset(selectionStartOffset - window.startOffset, text.length),
				selection.getDirection() === SelectionDirection.RTL
					? clampOffset(selectionStartOffset - window.startOffset, text.length)
					: clampOffset(selectionEndOffset - window.startOffset, text.length),
				selection,
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
			this.input.domNode.removeAttribute('aria-description');
			return;
		}
		const primary = selections.primary.getPosition();
		this.input.domNode.setAttribute(
			'aria-description',
			`${selections.selections.length} selections. Primary at line ${primary.lineNumber}, column ${primary.column}.`,
		);
	}

	private scheduleScreenReaderContent(): void {
		if (this.accessibleInputSyncScheduled) return;
		this.accessibleInputSyncScheduled = true;
		queueMicrotask(() => {
			this.accessibleInputSyncScheduled = false;
			this.writeScreenReaderContent('composition changed');
		});
	}

	private acceptAccessibleSelection(): void {
		if (this.compositionController.composing || this.input.domNode.ownerDocument.activeElement !== this.input.domNode) return;
		const model = this.viewport.textModel;
		this.accessibleInputState = TextAreaState.readFromTextArea(this.input, this.accessibleInputState);
		const startOffset = this.input.domNode.selectionStart;
		const endOffset = this.input.domNode.selectionEnd;
		const backward = this.input.domNode.selectionDirection === 'backward';
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
		if (model.offsetAt(current.getSelectionStart()) === safeAnchorOffset && model.offsetAt(current.getPosition()) === safeActiveOffset) return;
		this.selectionController.setSelections(SelectionSet.single(Selection.fromPositions(
			model.positionAt(safeAnchorOffset),
			model.positionAt(safeActiveOffset),
		)));
		this.viewport.revealPosition(this.selectionController.selections.primary.getPosition());
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
