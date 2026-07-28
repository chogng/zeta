import { addDisposableListener } from "../../dom.js";
import { getAriaAttribute, setAriaAttribute, setRole, } from "../aria/aria.js";
import { Emitter } from "../../../common/event.js";
import { IME } from "../../../common/ime.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
/** A text input foundation with events, focus control, and validation state. */
export class InputBox extends DisposableOwner {
    element;
    inputElement;
    #message;
    #onDidChange = this.own(new Emitter());
    #onDidFocus = this.own(new Emitter());
    #onDidBlur = this.own(new Emitter());
    #onKeyDown = this.own(new Emitter());
    #readOnly;
    onDidChange = this.#onDidChange.event;
    onDidFocus = this.#onDidFocus.event;
    onDidBlur = this.#onDidBlur.event;
    onKeyDown = this.#onKeyDown.event;
    constructor(options = {}) {
        super();
        const ownerDocument = options.ownerDocument ?? document;
        this.element = ownerDocument.createElement("div");
        this.element.className = "zeta-input-box";
        this.defer(() => this.element.remove());
        this.inputElement = ownerDocument.createElement("input");
        this.inputElement.type = options.type ?? "text";
        this.inputElement.placeholder = options.placeholder ?? "";
        this.inputElement.disabled = options.enabled === false;
        this.element.classList.toggle("is-disabled", this.inputElement.disabled);
        this.inputElement.autocomplete = "off";
        this.inputElement.autocapitalize = "off";
        this.inputElement.spellcheck = false;
        if (options.ariaLabel) {
            setAriaAttribute(this.inputElement, "label", options.ariaLabel);
        }
        setRole(this.inputElement, options.role);
        if (options.ariaAutoComplete) {
            setAriaAttribute(this.inputElement, "autocomplete", options.ariaAutoComplete);
        }
        if (options.ariaControls) {
            setAriaAttribute(this.inputElement, "controls", options.ariaControls);
        }
        if (options.ariaExpanded !== undefined) {
            setAriaAttribute(this.inputElement, "expanded", options.ariaExpanded);
        }
        this.#readOnly = options.readOnly ?? false;
        this.#message = ownerDocument.createElement("div");
        this.#message.id = `zeta-input-message-${inputBoxSequence++}`;
        this.#message.className = "zeta-input-box-message";
        setRole(this.#message, "alert");
        this.#message.hidden = true;
        this.element.append(this.inputElement, this.#message);
        this.#syncReadOnly();
        this.own(IME.onDidChange(() => this.#syncReadOnly()));
        this.own(addDisposableListener(this.inputElement, "input", () => this.#onDidChange.fire(this.value)));
        this.own(addDisposableListener(this.inputElement, "focus", () => {
            this.element.classList.add("is-focused");
            this.#onDidFocus.fire();
        }));
        this.own(addDisposableListener(this.inputElement, "blur", () => {
            this.element.classList.remove("is-focused");
            this.#onDidBlur.fire();
        }));
        this.own(addDisposableListener(this.inputElement, "keydown", (event) => this.#onKeyDown.fire(event)));
    }
    get value() {
        return this.inputElement.value;
    }
    set value(value) {
        if (this.inputElement.value === value)
            return;
        this.inputElement.value = value;
        this.#onDidChange.fire(value);
    }
    get placeholder() {
        return this.inputElement.placeholder;
    }
    set placeholder(value) {
        this.inputElement.placeholder = value;
    }
    get readOnly() {
        return this.#readOnly;
    }
    set readOnly(value) {
        this.#readOnly = value;
        this.#syncReadOnly();
    }
    get enabled() {
        return !this.inputElement.disabled;
    }
    set enabled(value) {
        this.inputElement.disabled = !value;
        this.element.classList.toggle("is-disabled", !value);
    }
    get ariaActiveDescendant() {
        return getAriaAttribute(this.inputElement, "activedescendant");
    }
    set ariaActiveDescendant(value) {
        if (value) {
            setAriaAttribute(this.inputElement, "activedescendant", value);
        }
        else {
            setAriaAttribute(this.inputElement, "activedescendant", undefined);
        }
    }
    focus() {
        this.inputElement.focus();
    }
    blur() {
        this.inputElement.blur();
    }
    hasFocus() {
        return this.inputElement.ownerDocument.activeElement === this.inputElement;
    }
    select(selection) {
        if (selection) {
            this.inputElement.setSelectionRange(selection.start, selection.end);
        }
        else {
            this.inputElement.select();
        }
    }
    showValidation(message) {
        this.#message.textContent = message;
        this.#message.hidden = !message;
        this.element.classList.toggle("has-validation", Boolean(message));
        if (message) {
            setAriaAttribute(this.inputElement, "invalid", true);
            setAriaAttribute(this.inputElement, "describedby", this.#message.id);
        }
        else {
            setAriaAttribute(this.inputElement, "invalid", undefined);
            setAriaAttribute(this.inputElement, "describedby", undefined);
        }
    }
    #syncReadOnly() {
        this.inputElement.readOnly = this.#readOnly || !IME.enabled;
    }
}
let inputBoxSequence = 1;
