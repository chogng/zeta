import { toDisposable } from "../../../../../base/common/lifecycle.js";
import { h } from "../../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../../base/browser/fastDomNode.js";
import { type Event } from "../../../../../base/common/event.js";
import { type EditContextCompositionEvent, EditContext, type EditContextOptions, type EditContextPosition, type EditContextState } from "../editContext.js";
import { TextAreaInput } from "./textAreaEditContextInput.js";
import { TextAreaEditContextRegistry } from "./textAreaEditContextRegistry.js";
import { type ITextAreaWrapper } from "./textAreaEditContextState.js";

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
		this.inputNode.setTabIndex(-1);
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
