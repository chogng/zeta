import { stopEvent } from "../../../../base/browser/dom.js";
import { StandardKeyboardEvent } from "../../../../base/browser/keyboardEvent.js";
import { Emitter, Event, type Event as EditorEvent } from "../../../../base/common/event.js";
import { IME } from "../../../../base/common/ime.js";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { type TextSelectionOffsets } from "../../../common/commands/editorEditCommand.js";
import { type CompositionSession } from '../../../common/cursor/cursor.js';
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { type Position } from "../../../common/core/position.js";
import { normalizeTextLineEndings } from "../../../common/core/textChange.js";
import { type IAccessibilityService } from '../../../../platform/accessibility/common/accessibility.js';
import { type IEditorAriaOptions } from '../../editorBrowser.js';
import { type View } from "../../view.js";
import { type EditorViewTextUpdateEvent, type ViewController } from "../../view/viewController.js";
import { type BracketColorizationSource, type SemanticTokenSource } from '../../viewParts/viewLines/viewLine.js';
import { createEditorClipboardCopyEvent, createClipboardPasteEvent, type IEditorClipboardCopyEvent, type IClipboardPasteEvent } from "./clipboardUtils.js";

/** The state that the browser editing surface mirrors from the common editor. */
export interface EditContextState {
	readonly text: string;
	readonly selectionStart: number;
	readonly selectionEnd: number;
	readonly position: Position;
}

/** A browser text update expressed independently of the concrete input element. */
export interface EditContextTextUpdate {
	readonly text: string;
	readonly updateRangeStart: number;
	readonly updateRangeEnd: number;
	readonly selectionStart: number;
	readonly selectionEnd: number;
	readonly previousSelectionStart: number;
	readonly previousSelectionEnd: number;
	readonly inputType: string | undefined;
}

/** A normalized composition event shared by textarea and native EditContext. */
export interface EditContextCompositionEvent {
	readonly data: string;
	readonly text: string;
	readonly selection: TextSelectionOffsets;
	readonly browserEvent: globalThis.Event;
}

/** Composition formatting reported by the native EditContext implementation. */
export interface EditContextTextFormat {
	readonly rangeStart: number;
	readonly rangeEnd: number;
	readonly underlineThickness: string;
}

export interface EditContextTextFormatUpdate {
	readonly formats: readonly EditContextTextFormat[];
	readonly browserEvent: globalThis.Event;
}

export interface EditContextCharacterBounds {
	readonly left: number;
	readonly top: number;
	readonly width: number;
	readonly height: number;
}

export interface EditContextOptions {
	readonly ariaLabel?: string;
	readonly readOnly: boolean;
	readonly textDirection: string;
	/** Stable editor-view identity used by host integrations. */
	readonly ownerId: string;
	/** Resolves model-relative character geometry for native IME requests. */
	readonly characterBoundsProvider: (
		modelOffset: number,
	) => EditContextCharacterBounds | undefined;
	readonly viewController: ViewController;
	readonly viewport: View;
	readonly selectionController: CursorsController;
	readonly accessibilityService?: IAccessibilityService;
	readonly renderRichScreenReaderContent?: boolean;
	readonly accessibilityPageSize?: number;
	readonly semanticTokenSource?: SemanticTokenSource;
	readonly bracketColorizationSource?: BracketColorizationSource;
}

/** Content coordinates of the primary editor caret. */
export interface EditContextPosition {
	readonly left: number;
	readonly top: number;
	readonly height: number;
}

/**
 * Browser editing surface used by the semantic editor-view collaborators.
 *
 * The common editor never talks directly to a textarea or to the browser's
 * EditContext object. Implementations translate their platform-specific DOM
 * events into this contract and mirror common-editor state back to the
 * browser. This is the same seam that lets VS Code provide native and
 * textarea edit contexts side by side.
 */
export abstract class AbstractEditContext extends Disposable {
	abstract readonly domNode: HTMLElement;
	abstract readonly textArea: HTMLTextAreaElement | undefined;
	private readonly willCopyEmitter = this._register(new Emitter<IEditorClipboardCopyEvent>());
	private readonly willCutEmitter = this._register(new Emitter<IEditorClipboardCopyEvent>());
	private readonly willPasteEmitter = this._register(new Emitter<IClipboardPasteEvent>());
	private readonly willBeforeInputEmitter = this._register(new Emitter<InputEvent>());
	private readonly willTextUpdateEmitter = this._register(new Emitter<EditorViewTextUpdateEvent>());
	private readonly willKeydownEmitter = this._register(new Emitter<KeyboardEvent>());
	private compositionControllerValue: CompositionController | undefined;
	abstract readonly onDidFocus: EditorEvent<void>;
	abstract readonly onDidBlur: EditorEvent<void>;
	abstract readonly onDidBeforeInput: EditorEvent<InputEvent>;
	abstract readonly onDidInput: EditorEvent<InputEvent>;
	readonly onWillBeforeInput: EditorEvent<InputEvent> = this.willBeforeInputEmitter.event;
	readonly onWillTextUpdate: EditorEvent<EditorViewTextUpdateEvent> = this.willTextUpdateEmitter.event;
	readonly onWillKeydown: EditorEvent<KeyboardEvent> = this.willKeydownEmitter.event;
	/** Native EditContext publishes this event; textarea uses an empty event. */
	readonly onDidTextUpdate: EditorEvent<EditContextTextUpdate> = Event.None;
	/** Native EditContext publishes composition formatting; textarea has no equivalent. */
	readonly onDidTextFormatUpdate: EditorEvent<EditContextTextFormatUpdate> = Event.None;
	abstract readonly onDidSelect: EditorEvent<void>;
	abstract readonly onDidKeydown: EditorEvent<KeyboardEvent>;
	abstract readonly onDidCompositionStart: EditorEvent<EditContextCompositionEvent>;
	abstract readonly onDidCompositionUpdate: EditorEvent<EditContextCompositionEvent>;
	abstract readonly onDidCompositionEnd: EditorEvent<EditContextCompositionEvent>;
	/** Clipboard events are emitted before the editor's clipboard contribution handles them. */
	readonly onWillCopy: EditorEvent<IEditorClipboardCopyEvent> = this.willCopyEmitter.event;
	readonly onWillCut: EditorEvent<IEditorClipboardCopyEvent> = this.willCutEmitter.event;
	readonly onWillPaste: EditorEvent<IClipboardPasteEvent> = this.willPasteEmitter.event;

	abstract get readOnly(): boolean;
	abstract connect(): void;
	abstract focus(): void;
	abstract isFocused(): boolean;
	abstract refreshFocusState(): void;
	abstract setAriaOptions(options: IEditorAriaOptions): void;
	abstract getLastRenderData(): Position | null;
	abstract writeScreenReaderContent(reason: string): void;
	/** Clears transient browser input after a routed DOM event. */
	abstract clear(): void;
	/** Mirrors the common model and primary selection into the browser surface. */
	abstract syncState(state: EditContextState): void;
	/** Updates native control and selection geometry for the primary caret. */
	abstract updateBounds(position: EditContextPosition): void;
	/** Starts the browser's composition presentation. */
	abstract prepareComposition(): void;
	/** Positions the browser's composition presentation in viewport content space. */
	abstract positionComposition(position: EditContextPosition): void;
	/** Removes transient composition presentation state. */
	abstract clearComposition(): void;
	/** Updates the read-only state when IME availability changes. */
	abstract setReadOnly(readOnly: boolean): void;

	get compositionController(): CompositionController {
		if (!this.compositionControllerValue) throw new ReferenceError('Edit context controller is not initialized');
		return this.compositionControllerValue;
	}

	protected initializeController(options: EditContextOptions): CompositionController {
		if (this.compositionControllerValue) throw new ReferenceError('Edit context controller is already initialized');
		const compositionController = this._register(new CompositionController(this, options.viewport, options.selectionController));
		this.compositionControllerValue = compositionController;
		const viewController = options.viewController;
		this._register(this.onDidBeforeInput(event => this.routeBeforeInput(event, viewController, compositionController)));
		this._register(this.onDidInput(event => {
			if (!event.isComposing || !compositionController.composing) this.clear();
		}));
		this._register(this.onDidTextUpdate(update => this.routeTextUpdate(update, viewController, compositionController)));
		this._register(this.onDidKeydown(event => this.routeKeydown(event, viewController)));
		return compositionController;
	}

	protected fireWillCopy(browserEvent: ClipboardEvent, isCut: boolean): void {
		const event = createEditorClipboardCopyEvent(browserEvent, isCut);
		(isCut ? this.willCutEmitter : this.willCopyEmitter).fire(event);
	}

	protected fireWillPaste(browserEvent: ClipboardEvent): void {
		this.willPasteEmitter.fire(createClipboardPasteEvent(browserEvent));
	}

	private routeBeforeInput(event: InputEvent, viewController: ViewController, compositionController: CompositionController): void {
		if (event.defaultPrevented || (event.isComposing && compositionController.composing)) return;
		this.willBeforeInputEmitter.fire(event);
		if (event.defaultPrevented) return;

		switch (event.inputType) {
			case "insertText":
			case "insertReplacementText":
				if (!event.data) return;
				stopEvent(event);
				this.clear();
				viewController.type(event.data, event.inputType);
				return;
			case "insertLineBreak":
			case "insertParagraph":
				stopEvent(event);
				this.clear();
				viewController.enter(event.inputType);
				return;
			case "deleteContentBackward":
				stopEvent(event);
				this.clear();
				viewController.deleteBackward(event.inputType);
				return;
			case "deleteContentForward":
				stopEvent(event);
				this.clear();
				viewController.deleteForward(event.inputType);
				return;
			case "deleteWordBackward":
				stopEvent(event);
				this.clear();
				viewController.deleteWordBackward(event.inputType);
				return;
			case "deleteWordForward":
				stopEvent(event);
				this.clear();
				viewController.deleteWordForward(event.inputType);
				return;
			case "deleteSoftLineBackward":
				stopEvent(event);
				this.clear();
				viewController.deleteSoftLineBackward(event.inputType);
				return;
			case "deleteSoftLineForward":
				stopEvent(event);
				this.clear();
				viewController.deleteSoftLineForward(event.inputType);
				return;
			case "historyUndo":
				stopEvent(event);
				this.clear();
				viewController.undo();
				return;
			case "historyRedo":
				stopEvent(event);
				this.clear();
				viewController.redo();
				return;
			default:
				return;
		}
	}

	private routeTextUpdate(update: EditContextTextUpdate, viewController: ViewController, compositionController: CompositionController): void {
		if (compositionController.composing) return;
		if (update.updateRangeStart === update.updateRangeEnd && update.text.length === 0) return;
		const event = makeTextUpdateEvent(update);
		this.willTextUpdateEmitter.fire(event);
		if (event.defaultPrevented) return;
		viewController.applyTextUpdate(update);
	}

	private routeKeydown(event: KeyboardEvent, viewController: ViewController): void {
		if (event.defaultPrevented) return;
		viewController.emitKeyDown(new StandardKeyboardEvent(event));
		this.willKeydownEmitter.fire(event);
		if (event.defaultPrevented) return;
		if (!event.isComposing && !event.getModifierState("AltGraph")) {
			if (isUndoKeybinding(event)) {
				stopEvent(event);
				this.clear();
				viewController.undo();
				return;
			}
			if (isRedoKeybinding(event)) {
				stopEvent(event);
				this.clear();
				viewController.redo();
				return;
			}
		}
		if (!event.isComposing && event.key === "Insert" && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
			stopEvent(event);
			viewController.toggleOvertype();
			return;
		}
		if (
			event.isComposing ||
			event.key !== "Tab" ||
			event.shiftKey ||
			event.ctrlKey ||
			event.altKey ||
			event.metaKey
		) return;
		if (viewController.hasExpandedSelections) return;
		stopEvent(event);
		// Tab is a keyboard command, not a browser text-input transaction. Keep it
		// out of post-edit consumers such as SuggestController, matching VS Code's
		// command dispatch ordering.
		viewController.insertTab();
	}
}

function makeTextUpdateEvent(update: EditContextTextUpdate): EditorViewTextUpdateEvent {
	let defaultPrevented = false;
	return {
		...update,
		get defaultPrevented(): boolean {
			return defaultPrevented;
		},
		preventDefault(): void {
			defaultPrevented = true;
		},
	};
}

function isUndoKeybinding(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">): boolean {
	return hasPrimaryModifier(event) && !event.shiftKey && event.key.toLowerCase() === "z";
}

function isRedoKeybinding(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">): boolean {
	if (!hasPrimaryModifier(event)) return false;
	const key = event.key.toLowerCase();
	return (key === "z" && event.shiftKey) || (key === "y" && !event.shiftKey);
}

function hasPrimaryModifier(event: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "metaKey">): boolean {
	return !event.altKey && event.ctrlKey !== event.metaKey;
}

interface ActiveComposition {
	readonly session: CompositionSession;
	text: string;
	selection: TextSelectionOffsets;
	updated: boolean;
	cancelRequested: boolean;
}

/**
 * Maps an edit-context composition stream to one protected Stanza composition session.
 */
export class CompositionController extends Disposable {
	private readonly _onDidChange = this._register(new Emitter<boolean>());
	private readonly input: AbstractEditContext;
	private readonly initialReadOnly: boolean;
	private activeComposition: ActiveComposition | undefined;

	readonly onDidChange: Event<boolean> = this._onDidChange.event;

	constructor(
		input: AbstractEditContext,
		private readonly viewport: View,
		private readonly selectionController: CursorsController,
	) {
		super();
		if (viewport.textModel !== selectionController.textModel) {
			this.dispose();
			throw new TypeError(
				"Stanza composition and selection controllers must share one text model",
			);
		}
		this.input = input;
		this.initialReadOnly = input.readOnly;
		this._register(toDisposable(() => {
			this.cancelComposition();
			this.input.setReadOnly(this.initialReadOnly);
			this.clearPresentation();
		}));
		this._register(input.onDidCompositionStart(event => this.handleCompositionStart(event)));
		this._register(input.onDidCompositionUpdate(event => this.handleCompositionUpdate(event)));
		this._register(input.onDidCompositionEnd(event => this.handleCompositionEnd(event)));
		this._register(input.onDidKeydown(event => {
			if (event.isComposing && event.key === "Escape" && this.activeComposition) {
				this.activeComposition.cancelRequested = true;
			}
		}));
		this._register(input.onDidBlur(() => this.cancelComposition()));
		this._register(IME.onDidChange(enabled => {
			if (!enabled) this.cancelComposition();
			this.synchronizeReadOnly();
		}));
		this._register(selectionController.onDidChange(() => {
			this.finishInvalidComposition();
		}));
		this._register(viewport.textModel.onDidChange(() => {
			this.finishInvalidComposition();
		}));
		this._register(viewport.onDidChangeLayout(() => {
			if (this.activeComposition) this.positionInputAtPrimary();
		}));
		this.synchronizeReadOnly();
	}

	get composing(): boolean {
		return Boolean(this.activeComposition?.session.active);
	}

	private handleCompositionStart(event: EditContextCompositionEvent): void {
		if (event.browserEvent.defaultPrevented || this.activeComposition) return;
		if (
			!IME.enabled ||
			this.selectionController.readOnly ||
			this.selectionController.selections.selections.length !== 1
		) {
			event.browserEvent.preventDefault();
			return;
		}
		this.input.prepareComposition();
		const startPosition = this.selectionController.selections.primary.getStartPosition();
		const session = this.selectionController.beginComposition();
		this.activeComposition = {
			session,
			text: "",
			selection: { anchorOffset: 0, activeOffset: 0 },
			updated: false,
			cancelRequested: false,
		};
		this.viewport.element.classList.add("composing");
		this._onDidChange.fire(true);
		this.viewport.revealPosition(startPosition);
		this.positionInput(startPosition);
	}

	private handleCompositionUpdate(event: EditContextCompositionEvent): void {
		if (event.browserEvent.defaultPrevented) return;
		this.updateComposition(event.text, event.selection);
	}

	private handleCompositionEnd(event: EditContextCompositionEvent): void {
		const active = this.activeComposition;
		if (!active) return;
		if (!active.session.active) {
			this.activeComposition = undefined;
			this.finishPresentation();
			return;
		}
		if (active.cancelRequested) {
			this.cancelComposition();
			return;
		}
		this.updateComposition(event.text, event.selection);
		const current = this.activeComposition;
		if (!current?.session.active) {
			this.finishPresentation();
			return;
		}
		this.activeComposition = undefined;
		current.session.commit();
		this.finishPresentation();
	}

	private updateComposition(text: string, selection: TextSelectionOffsets): void {
		const active = this.activeComposition;
		if (!active) return;
		if (!active.session.active) {
			this.activeComposition = undefined;
			this.finishPresentation();
			return;
		}
		text = normalizeTextLineEndings(text);
		if (
			active.updated &&
			active.text === text &&
			selectionsEqual(active.selection, selection)
		) {
			this.positionInputAtPrimary();
			return;
		}
		try {
			active.session.update({ text, selection });
		} catch (error) {
			if (!active.session.active) {
				this.finishInvalidComposition();
				return;
			}
			throw error;
		}
		if (this.activeComposition !== active || !active.session.active) {
			this.finishInvalidComposition();
			return;
		}
		active.text = text;
		active.selection = selection;
		active.updated = true;
		this.viewport.setCompositionRange(active.session.currentRange);
		this.viewport.revealPosition(
			this.selectionController.selections.primary.getPosition(),
		);
		this.positionInputAtPrimary();
	}

	private cancelComposition(): void {
		const active = this.activeComposition;
		if (!active) return;
		this.activeComposition = undefined;
		if (active.session.active) active.session.cancel();
		this.finishPresentation();
	}

	private finishInvalidComposition(): void {
		const active = this.activeComposition;
		if (!active || active.session.active) return;
		this.activeComposition = undefined;
		this.finishPresentation();
	}

	private positionInputAtPrimary(): void {
		this.positionInput(this.selectionController.selections.primary.getPosition());
	}

	private positionInput(position: Position): void {
		const coordinates = this.viewport.getPositionContentCoordinates(position);
		this.input.positionComposition(coordinates);
	}

	private finishPresentation(): void {
		const changed = this.clearPresentation();
		if (changed) this._onDidChange.fire(false);
	}

	private clearPresentation(): boolean {
		const changed = this.viewport.element.classList.contains("composing") ||
			this.input.domNode.classList.contains("ime-input");
		this.viewport.element.classList.remove("composing");
		this.input.clearComposition();
		this.viewport.setCompositionRange(undefined);
		return changed;
	}

	private synchronizeReadOnly(): void {
		this.input.setReadOnly(this.initialReadOnly || !IME.enabled);
	}
}

function selectionsEqual(left: TextSelectionOffsets, right: TextSelectionOffsets): boolean {
	return left.anchorOffset === right.anchorOffset &&
		left.activeOffset === right.activeOffset;
}
