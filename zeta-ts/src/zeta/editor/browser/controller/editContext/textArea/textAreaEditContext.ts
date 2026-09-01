import './textAreaEditContext.css';
import { Disposable } from "../../../../../base/common/lifecycle.js";
import { h } from "../../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../../base/browser/fastDomNode.js";
import { type Event } from "../../../../../base/common/event.js";
import { AbstractEditContext, type CompositionController, type EditContextCompositionEvent, type EditContextOptions, type EditContextPosition, type EditContextState } from "../editContext.js";
import { type IEditorAriaOptions } from '../../../editorBrowser.js';
import { type CursorsController } from '../../../../common/cursor/cursor.js';
import { SelectionDirection, Selection } from '../../../../common/core/selection.js';
import { Position } from '../../../../common/core/position.js';
import { Range } from '../../../../common/core/range.js';
import { type TextModel } from '../../../../common/model/textModel.js';
import { type View } from '../../../view.js';
import { type RenderingContext, type RestrictedRenderingContext } from '../../../view/renderingContext.js';
import { type ViewContext } from '../../../../common/viewModel/viewContext.js';
import * as viewEvents from '../../../../common/viewEvents.js';
import { EditorOption } from '../../../../common/config/editorOptions.js';
import { MappedScreenReaderStrategy, modelOffsetAtContentOffset, type MappedScreenReaderContentState } from '../screenReaderUtils.js';
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
	readonly textArea: FastDomNode<HTMLTextAreaElement>;
	readonly textAreaInput: TextAreaInput;
	private readonly accessibilityController: TextAreaAccessibilityController;
	private lastRenderPosition: Position | null = null;
	private renderPosition: EditContextPosition | undefined;
	private connected = false;

	get onDidFocus(): Event<void> { return this.textAreaInput.onDidFocus; }
	get onDidBlur(): Event<void> { return this.textAreaInput.onDidBlur; }
	get onDidBeforeInput(): Event<InputEvent> { return this.textAreaInput.onDidBeforeInput; }
	get onDidInput(): Event<InputEvent> { return this.textAreaInput.onDidInput; }
	get onDidSelect(): Event<void> { return this.textAreaInput.onDidSelect; }
	get onDidKeydown(): Event<KeyboardEvent> { return this.textAreaInput.onDidKeydown; }
	get onDidKeyup(): Event<KeyboardEvent> { return this.textAreaInput.onDidKeyup; }
	get onDidCompositionStart(): Event<EditContextCompositionEvent> { return this.textAreaInput.onDidCompositionStart; }
	get onDidCompositionUpdate(): Event<EditContextCompositionEvent> { return this.textAreaInput.onDidCompositionUpdate; }
	get onDidCompositionEnd(): Event<EditContextCompositionEvent> { return this.textAreaInput.onDidCompositionEnd; }

	constructor(
		context: ViewContext,
		container: HTMLElement,
		options: TextAreaEditContextOptions,
	) {
		super(context);
		const ownerDocument = container.ownerDocument;
		this.textArea = new FastDomNode(h(ownerDocument, "textarea"));
		this.textArea.setClassName("stanza-editor-input");
		this.textArea.domNode.tabIndex = -1;
		this.textArea.domNode.spellcheck = false;
		this.textArea.domNode.readOnly = options.readOnly;
		this.textArea.domNode.wrap = "off";
		this.textArea.domNode.dir = options.textDirection;
		this.textArea.domNode.autocomplete = "off";
		this.textArea.setAttribute("autocapitalize", "off");
		this.textArea.setAttribute("aria-label", options.ariaLabel ?? "Stanza editor input");
		this.textArea.setAttribute("aria-multiline", "true");
		this.textArea.setAttribute("aria-roledescription", "code editor");
		this.textArea.setAttribute("aria-readonly", String(this.textArea.domNode.readOnly));
		this.textAreaInput = this._register(new TextAreaInput(this.textArea.domNode));
		this._register(TextAreaEditContextRegistry.register(options.ownerId, this));
		const compositionController = this.initializeController(options);
		this.accessibilityController = this._register(new TextAreaAccessibilityController(this, options.viewport, options.selectionController, compositionController));
		this.synchronizeState();
		this.connect();
	}

	get domNode(): FastDomNode<HTMLElement> {
		return this.textArea;
	}

	get readOnly(): boolean {
		return this.textArea.domNode.readOnly;
	}

	/**
	 * Installs DOM listeners after higher-level consumers have subscribed to the
	 * edit-context events. This preserves completion and clipboard ordering.
	 */
	private connect(): void {
		this.assertNotDisposed();
		if (this.connected) return;
		this.connected = true;
		this._register(this.textAreaInput.onDidCopy(event => this.fireWillCopy(event, false)));
		this._register(this.textAreaInput.onDidCut(event => this.fireWillCopy(event, true)));
		this._register(this.textAreaInput.onDidPaste(event => this.fireWillPaste(event)));
		this.textAreaInput.connect();
	}

	focus(): void {
		this.textAreaInput.focusTextArea();
	}

	isFocused(): boolean {
		return this.textAreaInput.isFocused();
	}

	refreshFocusState(): void {
		this.textAreaInput.refreshFocusState();
	}

	setAriaOptions(options: IEditorAriaOptions): void {
		if (options.activeDescendant) {
			this.textArea.setAttribute('aria-haspopup', 'true');
			this.textArea.setAttribute('aria-autocomplete', 'list');
			this.textArea.setAttribute('aria-activedescendant', options.activeDescendant);
		} else {
			this.textArea.setAttribute('aria-haspopup', 'false');
			this.textArea.setAttribute('aria-autocomplete', 'both');
			this.textArea.removeAttribute('aria-activedescendant');
		}
		if (options.role) this.textArea.setAttribute('role', options.role);
	}

	getLastRenderData(): Position | null {
		return this.lastRenderPosition;
	}

	public getTextAreaDomNode(): HTMLTextAreaElement {
		return this.textArea.domNode;
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
		this.textArea.domNode.readOnly = readOnly;
		this.textArea.setAttribute("aria-readonly", String(readOnly));
	}

	prepareComposition(): void {
		this.textAreaInput.clear();
		this.textArea.toggleClassName("ime-input", true);
	}

	positionComposition(position: EditContextPosition): void {
		this.textArea.setLeft(position.left);
		this.textArea.setTop(position.top);
		this.textArea.setHeight(position.height);
	}

	clearComposition(): void {
		this.textAreaInput.clear();
		this.textArea.toggleClassName("ime-input", false);
		this.textArea.setLeft("");
		this.textArea.setTop("");
		this.textArea.setHeight("");
	}

	public override onConfigurationChanged(event: viewEvents.ViewConfigurationChangedEvent): boolean {
		if (event.hasChanged(EditorOption.readOnly)) {
			this.setReadOnly(this._context.configuration.options.get(EditorOption.readOnly));
		}
		if (event.hasChanged(EditorOption.ariaLabel)) {
			this.textArea.setAttribute('aria-label', this._context.configuration.options.get(EditorOption.ariaLabel));
		}
		return event.hasChanged(EditorOption.readOnly) || event.hasChanged(EditorOption.ariaLabel);
	}

	public override onCursorStateChanged(_event: viewEvents.ViewCursorStateChangedEvent): boolean {
		this.synchronizeState();
		return true;
	}

	public override onDecorationsChanged(_event: viewEvents.ViewDecorationsChangedEvent): boolean {
		return true;
	}

	public override onFlushed(_event: viewEvents.ViewFlushedEvent): boolean {
		this.synchronizeState();
		return true;
	}

	public override onLineMappingChanged(_event: viewEvents.ViewLineMappingChangedEvent): boolean {
		this.synchronizeState();
		return true;
	}

	public override onLinesChanged(_event: viewEvents.ViewLinesChangedEvent): boolean {
		this.synchronizeState();
		return true;
	}

	public override onLinesDeleted(_event: viewEvents.ViewLinesDeletedEvent): boolean {
		this.synchronizeState();
		return true;
	}

	public override onLinesInserted(_event: viewEvents.ViewLinesInsertedEvent): boolean {
		this.synchronizeState();
		return true;
	}

	public override onScrollChanged(_event: viewEvents.ViewScrollChangedEvent): boolean {
		return true;
	}

	public override onZonesChanged(_event: viewEvents.ViewZonesChangedEvent): boolean {
		return true;
	}

	public override prepareRender(_context: RenderingContext): void {
		this.renderPosition = this.readPosition();
	}

	public override render(_context: RestrictedRenderingContext): void {
		if (this.renderPosition) this.updateBounds(this.renderPosition);
		this.writeScreenReaderContent('render');
	}

	public override dispose(): void {
		this.textArea.domNode.remove();
		super.dispose();
	}
}

const ACCESSIBILITY_LINES_PER_PAGE = 500;
const MAXIMUM_ACCESSIBLE_INPUT_TEXT_UNITS = 32 * 1_024;

/** Mirrors the active editor window into the native input for assistive technology. */
class TextAreaAccessibilityController extends Disposable {
	private accessibleInputSyncScheduled = false;
	private accessibleInputState = TextAreaState.EMPTY;
	private accessibleScreenReaderContentState: MappedScreenReaderContentState | undefined;
	private accessibleInputStartOffset = 0;
	private readonly screenReaderStrategy = new MappedScreenReaderStrategy();

	constructor(
		private readonly input: TextAreaEditContext,
		private readonly viewport: View,
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
		const input = this.input.getTextAreaDomNode();
		if (this.isDisposed || this.compositionController.composing || input.ownerDocument.activeElement !== input) return;
		const model = this.viewport.textModel;
		const selection = this.selectionController.selections[0]!;
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
		if (selections.length === 1) {
			this.input.domNode.removeAttribute('aria-description');
			return;
		}
		const primary = selections[0]!.getPosition();
		this.input.domNode.setAttribute(
			'aria-description',
			`${selections.length} selections. Primary at line ${primary.lineNumber}, column ${primary.column}.`,
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
		const input = this.input.getTextAreaDomNode();
		if (this.compositionController.composing || input.ownerDocument.activeElement !== input) return;
		const model = this.viewport.textModel;
		this.accessibleInputState = TextAreaState.readFromTextArea(this.input, this.accessibleInputState);
		const startOffset = input.selectionStart;
		const endOffset = input.selectionEnd;
		const backward = input.selectionDirection === 'backward';
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
		const current = this.selectionController.selections[0]!;
		if (model.offsetAt(current.getSelectionStart()) === safeAnchorOffset && model.offsetAt(current.getPosition()) === safeActiveOffset) return;
		this.selectionController.setSelections([Selection.fromPositions(
			model.positionAt(safeAnchorOffset),
			model.positionAt(safeActiveOffset),
		)]);
		this.viewport.revealPosition(this.selectionController.selections[0]!.getPosition());
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
