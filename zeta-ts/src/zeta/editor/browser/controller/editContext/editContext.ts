import { stopEvent } from "../../../../base/browser/dom.js";
import { Emitter, type Event as EditorEvent, noEvent } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextSelectionOffsets } from "../../../common/commands/editorEditCommand.js";
import { type CompositionController } from "../compositionController.js";
import { type EditorViewTextUpdateEvent, type ViewController } from "../../view/viewController.js";
import { createClipboardCopyEvent, createClipboardPasteEvent, type IClipboardCopyEvent, type IClipboardPasteEvent } from "./clipboardUtils.js";

/** The state that the browser editing surface mirrors from the common editor. */
export interface EditContextState {
	readonly text: string;
	readonly selectionStart: number;
	readonly selectionEnd: number;
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
	readonly readOnly?: boolean;
	readonly textDirection?: string;
	/** Stable editor-view identity used by host integrations. */
	readonly ownerId?: string;
	/** Resolves model-relative character geometry for native IME requests. */
	readonly characterBoundsProvider?: (
		modelOffset: number,
	) => EditContextCharacterBounds | undefined;
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
export abstract class EditContext extends DisposableOwner {
	abstract readonly element: HTMLElement;
	abstract readonly textArea: HTMLTextAreaElement | undefined;
	private readonly willCopyEmitter = this.own(new Emitter<IClipboardCopyEvent>());
	private readonly willCutEmitter = this.own(new Emitter<IClipboardCopyEvent>());
	private readonly willPasteEmitter = this.own(new Emitter<IClipboardPasteEvent>());
	private readonly willBeforeInputEmitter = this.own(new Emitter<InputEvent>());
	private readonly willTextUpdateEmitter = this.own(new Emitter<EditorViewTextUpdateEvent>());
	private readonly willKeydownEmitter = this.own(new Emitter<KeyboardEvent>());
	private inputConnected = false;
	abstract readonly onDidFocus: EditorEvent<void>;
	abstract readonly onDidBlur: EditorEvent<void>;
	abstract readonly onDidBeforeInput: EditorEvent<InputEvent>;
	abstract readonly onDidInput: EditorEvent<InputEvent>;
	readonly onWillBeforeInput: EditorEvent<InputEvent> = this.willBeforeInputEmitter.event;
	readonly onWillTextUpdate: EditorEvent<EditorViewTextUpdateEvent> = this.willTextUpdateEmitter.event;
	readonly onWillKeydown: EditorEvent<KeyboardEvent> = this.willKeydownEmitter.event;
	/** Native EditContext publishes this event; textarea uses an empty event. */
	readonly onDidTextUpdate: EditorEvent<EditContextTextUpdate> = noEvent;
	/** Native EditContext publishes composition formatting; textarea has no equivalent. */
	readonly onDidTextFormatUpdate: EditorEvent<EditContextTextFormatUpdate> = noEvent;
	abstract readonly onDidSelect: EditorEvent<void>;
	abstract readonly onDidKeydown: EditorEvent<KeyboardEvent>;
	abstract readonly onDidCompositionStart: EditorEvent<EditContextCompositionEvent>;
	abstract readonly onDidCompositionUpdate: EditorEvent<EditContextCompositionEvent>;
	abstract readonly onDidCompositionEnd: EditorEvent<EditContextCompositionEvent>;
	/** Clipboard events are emitted before the editor's clipboard contribution handles them. */
	readonly onWillCopy: EditorEvent<IClipboardCopyEvent> = this.willCopyEmitter.event;
	readonly onWillCut: EditorEvent<IClipboardCopyEvent> = this.willCutEmitter.event;
	readonly onWillPaste: EditorEvent<IClipboardPasteEvent> = this.willPasteEmitter.event;

	abstract get readOnly(): boolean;
	abstract connect(): void;
	abstract focus(): void;
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

	/**
	 * Connects the platform input surface to the existing view command boundary.
	 *
	 * VS Code passes its ViewController into each concrete edit-context adapter;
	 * this shared hook keeps the same ownership model for both Zeta adapters
	 * without introducing a second input-controller file.
	 */
	connectViewController(viewController: ViewController, compositionController: CompositionController): void {
		if (this.inputConnected) return;
		this.inputConnected = true;
		this.own(this.onDidBeforeInput(event => this.routeBeforeInput(event, viewController, compositionController)));
		this.own(this.onDidInput(event => {
			if (!event.isComposing || !compositionController.composing) this.clear();
		}));
		this.own(this.onDidTextUpdate(update => this.routeTextUpdate(update, viewController, compositionController)));
		this.own(this.onDidKeydown(event => this.routeKeydown(event, viewController)));
	}

	protected fireWillCopy(browserEvent: ClipboardEvent, isCut: boolean): void {
		const event = createClipboardCopyEvent(browserEvent, isCut);
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
		viewController.emitKeyDown(event);
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
