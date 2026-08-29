import { addDisposableListener } from "../../../../../base/browser/dom.js";
import { Emitter, type Event } from "../../../../../base/common/event.js";
import { Disposable } from "../../../../../base/common/lifecycle.js";
import { type TextSelectionOffsets } from "../../../../common/commands/editorEditCommand.js";
import { normalizeTextLineEndings } from "../../../../common/core/textChange.js";
import { type EditContextCompositionEvent } from "../editContext.js";
import { type ITextAreaWrapper, TextAreaState } from "./textAreaEditContextState.js";

/**
 * DOM event bridge for the textarea edit-context backend.
 *
 * This class deliberately knows about the textarea only. Model edits are
 * routed by the owning EditorView, while accessibility content
 * is written by the textarea accessibility controller.
 */
export class TextAreaInput extends Disposable implements ITextAreaWrapper {
	private readonly focusEmitter = this._register(new Emitter<void>());
	private readonly blurEmitter = this._register(new Emitter<void>());
	private readonly beforeInputEmitter = this._register(new Emitter<InputEvent>());
	private readonly inputEmitter = this._register(new Emitter<InputEvent>());
	private readonly selectEmitter = this._register(new Emitter<void>());
	private readonly keydownEmitter = this._register(new Emitter<KeyboardEvent>());
	private readonly compositionStartEmitter = this._register(new Emitter<EditContextCompositionEvent>());
	private readonly compositionUpdateEmitter = this._register(new Emitter<EditContextCompositionEvent>());
	private readonly compositionEndEmitter = this._register(new Emitter<EditContextCompositionEvent>());
	private readonly copyEmitter = this._register(new Emitter<ClipboardEvent>());
	private readonly cutEmitter = this._register(new Emitter<ClipboardEvent>());
	private readonly pasteEmitter = this._register(new Emitter<ClipboardEvent>());
	private connected = false;
	private focused = false;
	private selectionChangeIgnoredUntil = 0;
	private _textAreaState = TextAreaState.EMPTY;

	readonly onDidFocus: Event<void> = this.focusEmitter.event;
	readonly onDidBlur: Event<void> = this.blurEmitter.event;
	readonly onDidBeforeInput: Event<InputEvent> = this.beforeInputEmitter.event;
	readonly onDidInput: Event<InputEvent> = this.inputEmitter.event;
	readonly onDidSelect: Event<void> = this.selectEmitter.event;
	readonly onDidKeydown: Event<KeyboardEvent> = this.keydownEmitter.event;
	readonly onDidCompositionStart: Event<EditContextCompositionEvent> = this.compositionStartEmitter.event;
	readonly onDidCompositionUpdate: Event<EditContextCompositionEvent> = this.compositionUpdateEmitter.event;
	readonly onDidCompositionEnd: Event<EditContextCompositionEvent> = this.compositionEndEmitter.event;
	readonly onDidCopy: Event<ClipboardEvent> = this.copyEmitter.event;
	readonly onDidCut: Event<ClipboardEvent> = this.cutEmitter.event;
	readonly onDidPaste: Event<ClipboardEvent> = this.pasteEmitter.event;

	constructor(readonly element: HTMLTextAreaElement) {
		super();
		this._textAreaState = TextAreaState.readFromTextArea(this, null);
	}

	get textAreaState(): TextAreaState {
		return this._textAreaState;
	}

	connect(): void {
		this.assertNotDisposed();
		if (this.connected) return;
		this.connected = true;
		this._register(addDisposableListener(this.element, "focus", () => this.setFocused(true)));
		this._register(addDisposableListener(this.element, "blur", () => this.setFocused(false)));
		this._register(addDisposableListener<CompositionEvent>(
			this.element,
			"compositionstart",
			event => this.compositionStartEmitter.fire(toCompositionEvent(this.element, event)),
		));
		this._register(addDisposableListener<CompositionEvent>(
			this.element,
			"compositionupdate",
			event => this.compositionUpdateEmitter.fire(toCompositionEvent(this.element, event)),
		));
		this._register(addDisposableListener<CompositionEvent>(
			this.element,
			"compositionend",
			event => this.compositionEndEmitter.fire(toCompositionEvent(this.element, event)),
		));
		this._register(addDisposableListener<InputEvent>(
			this.element,
			"beforeinput",
			event => this.beforeInputEmitter.fire(event),
		));
		this._register(addDisposableListener<InputEvent>(
			this.element,
			"input",
			event => {
				this._textAreaState = TextAreaState.readFromTextArea(this, this._textAreaState);
				this.inputEmitter.fire(event);
			},
		));
		this._register(addDisposableListener(this.element, "select", () => this.fireSelectionChange()));
		this._register(addDisposableListener(this.element.ownerDocument, "selectionchange", () => {
			if (this.element.ownerDocument.activeElement === this.element) this.fireSelectionChange();
		}));
		this._register(addDisposableListener<ClipboardEvent>(this.element, "copy", event => this.copyEmitter.fire(event)));
		this._register(addDisposableListener<ClipboardEvent>(this.element, "cut", event => this.cutEmitter.fire(event)));
		this._register(addDisposableListener<ClipboardEvent>(this.element, "paste", event => this.pasteEmitter.fire(event)));
		this._register(addDisposableListener<KeyboardEvent>(
			this.element,
			"keydown",
			event => this.keydownEmitter.fire(event),
		));
	}

	focus(): void {
		this.element.focus({ preventScroll: true });
		this.refreshFocusState();
	}

	isFocused(): boolean {
		return this.focused;
	}

	refreshFocusState(): void {
		this.setFocused(this.hasFocus());
	}

	clear(): void {
		this.setValue("clear", "");
	}

	getValue(): string {
		return this.element.value;
	}

	setValue(reason: string, value: string): void {
		if (this.element.value !== value) this.element.value = value;
		this._textAreaState = TextAreaState.readFromTextArea(this, this._textAreaState);
		void reason;
	}

	getSelectionStart(): number {
		return this.element.selectionDirection === "backward"
			? this.element.selectionEnd
			: this.element.selectionStart;
	}

	getSelectionEnd(): number {
		return this.element.selectionDirection === "backward"
			? this.element.selectionStart
			: this.element.selectionEnd;
	}

	setSelectionRange(reason: string, selectionStart: number, selectionEnd: number): void {
		const direction = selectionStart > selectionEnd ? "backward" : "forward";
		this.element.setSelectionRange(
			Math.min(selectionStart, selectionEnd),
			Math.max(selectionStart, selectionEnd),
			direction,
		);
		this._textAreaState = TextAreaState.readFromTextArea(this, this._textAreaState);
		void reason;
	}

	private setFocused(focused: boolean): void {
		if (this.focused === focused) return;
		this.focused = focused;
		(focused ? this.focusEmitter : this.blurEmitter).fire(undefined);
	}

	hasFocus(): boolean {
		if (!this.element.isConnected) return false;
		const root = this.element.getRootNode() as Document | ShadowRoot;
		const activeElement = 'activeElement' in root ? root.activeElement : this.element.ownerDocument.activeElement;
		return activeElement === this.element;
	}

	setIgnoreSelectionChangeTime(_reason: string): void {
		this.selectionChangeIgnoredUntil = Date.now() + 100;
	}

	getIgnoreSelectionChangeTime(): number {
		return this.selectionChangeIgnoredUntil;
	}

	resetSelectionChangeTime(): void {
		this.selectionChangeIgnoredUntil = 0;
	}

	writeState(reason: string, state: TextAreaState, select: boolean): void {
		state.writeToTextArea(reason, this, select);
		this._textAreaState = state;
	}

	private fireSelectionChange(): void {
		if (Date.now() < this.selectionChangeIgnoredUntil) return;
		this._textAreaState = TextAreaState.readFromTextArea(this, this._textAreaState);
		this.selectEmitter.fire(undefined);
	}
}

function toCompositionEvent(element: HTMLTextAreaElement, browserEvent: CompositionEvent): EditContextCompositionEvent {
	const rawText = element.value;
	const text = normalizeTextLineEndings(rawText);
	return {
		data: normalizeTextLineEndings(browserEvent.data ?? ""),
		text,
		selection: readCompositionSelection(element, rawText, text),
		browserEvent,
	};
}

function readCompositionSelection(element: HTMLTextAreaElement, rawText: string, normalizedText: string): TextSelectionOffsets {
	if (element.value !== rawText) {
		return {
			anchorOffset: normalizedText.length,
			activeOffset: normalizedText.length,
		};
	}
	const start = normalizedOffset(rawText, element.selectionStart);
	const end = normalizedOffset(rawText, element.selectionEnd);
	return element.selectionDirection === "backward"
		? { anchorOffset: end, activeOffset: start }
		: { anchorOffset: start, activeOffset: end };
}

function normalizedOffset(text: string, offset: number): number {
	return normalizeTextLineEndings(text.slice(0, offset)).length;
}
