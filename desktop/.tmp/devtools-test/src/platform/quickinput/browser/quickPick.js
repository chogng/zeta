import { stopEvent } from "../../../base/browser/dom.js";
import { setAriaAttribute, setRole, } from "../../../base/browser/ui/aria/aria.js";
import { InputBox, } from "../../../base/browser/ui/inputbox/inputbox.js";
import { Emitter } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { QuickInputList } from "./quickInputList.js";
/** DOM implementation of one searchable Quick Pick controller. */
export class BrowserQuickPick extends DisposableOwner {
    element;
    #inputBox;
    #list;
    #onDidAccept = this.own(new Emitter());
    #onDidChangeValue = this.own(new Emitter());
    #onDidHide = this.own(new Emitter());
    #options;
    #visible = false;
    #placeholder = "";
    onDidAccept = this.#onDidAccept.event;
    onDidChangeValue = this.#onDidChangeValue.event;
    onDidHide = this.#onDidHide.event;
    constructor(options) {
        super();
        this.#options = options;
        const ownerDocument = options.ownerDocument;
        this.element = ownerDocument.createElement("div");
        this.element.className = "zeta-quick-pick";
        setRole(this.element, "dialog");
        setAriaAttribute(this.element, "label", "Quick Pick");
        this.defer(() => {
            if (this.#visible)
                this.hide();
            options.onDispose(this);
            this.element.remove();
        });
        this.#list = this.own(new QuickInputList(ownerDocument));
        this.#inputBox = this.own(new InputBox({
            ownerDocument,
            type: "search",
            ariaLabel: "Quick Pick",
            role: "combobox",
            ariaAutoComplete: "list",
            ariaControls: this.#list.listId,
            ariaExpanded: true,
        }));
        this.#inputBox.element.classList.add("zeta-quick-pick-input");
        this.element.append(this.#inputBox.element, this.#list.element);
        this.own(this.#inputBox.onDidChange((value) => this.#handleValueChange(value)));
        this.own(this.#list.onDidAccept((item) => {
            this.#onDidAccept.fire(item);
        }));
        this.own(this.#list.onDidChangeActive(({ rowId }) => {
            this.#inputBox.ariaActiveDescendant = rowId;
        }));
        this.own(this.#inputBox.onKeyDown((event) => this.#handleKeyDown(event)));
    }
    get items() {
        return this.#list.items;
    }
    set items(items) {
        this.#list.items = items;
    }
    get placeholder() {
        return this.#placeholder;
    }
    set placeholder(value) {
        this.#placeholder = value;
        this.#inputBox.placeholder = value;
    }
    get value() {
        return this.#inputBox.value;
    }
    set value(value) {
        this.#inputBox.value = value;
    }
    show() {
        if (this.#visible) {
            this.focus();
            return;
        }
        this.#visible = true;
        this.#options.onShow(this);
        this.focus();
    }
    hide() {
        if (!this.#visible)
            return;
        this.#visible = false;
        this.#options.onHide(this);
        this.#onDidHide.fire();
    }
    focus() {
        this.#inputBox.focus();
        this.#inputBox.select();
    }
    #handleValueChange(value) {
        this.#list.filter(value);
        this.#onDidChangeValue.fire(value);
    }
    #handleKeyDown(event) {
        switch (event.key) {
            case "ArrowDown":
                stopEvent(event);
                this.#list.focusNext();
                break;
            case "ArrowUp":
                stopEvent(event);
                this.#list.focusPrevious();
                break;
            case "Enter":
                stopEvent(event);
                this.#list.acceptActive();
                break;
            case "Escape":
                stopEvent(event);
                this.hide();
                break;
        }
    }
}
