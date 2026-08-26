import { Emitter, type Event as EditorEvent, noEvent } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextSelectionOffsets } from "../../../common/commands/editorEditCommand.js";
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
	abstract readonly onDidFocus: EditorEvent<void>;
	abstract readonly onDidBlur: EditorEvent<void>;
	abstract readonly onDidBeforeInput: EditorEvent<InputEvent>;
	abstract readonly onDidInput: EditorEvent<InputEvent>;
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

	protected fireWillCopy(browserEvent: ClipboardEvent, isCut: boolean): void {
		const event = createClipboardCopyEvent(browserEvent, isCut);
		(isCut ? this.willCutEmitter : this.willCopyEmitter).fire(event);
	}

	protected fireWillPaste(browserEvent: ClipboardEvent): void {
		this.willPasteEmitter.fire(createClipboardPasteEvent(browserEvent));
	}
}
