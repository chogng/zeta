import { addDisposableListener, isHTMLElement, stopEvent, } from "../../dom.js";
import { setAriaAttribute, setRole, } from "../aria/aria.js";
import { Emitter } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
/**
 * A single-focus list foundation with rendering, navigation, mouse acceptance,
 * and listbox accessibility. Product-specific filtering stays with callers.
 */
export class List extends DisposableOwner {
    element;
    #renderItem;
    #loopNavigation;
    #onDidChangeActive = this.own(new Emitter());
    #onDidAccept = this.own(new Emitter());
    #items = [];
    #activeIndex = -1;
    onDidChangeActive = this.#onDidChangeActive.event;
    onDidAccept = this.#onDidAccept.event;
    constructor(options) {
        super();
        const ownerDocument = options.ownerDocument ?? document;
        this.#renderItem = options.renderItem;
        this.#loopNavigation = options.loopNavigation ?? true;
        this.element = ownerDocument.createElement("div");
        this.element.className = "zeta-list";
        this.element.id = `zeta-list-${listSequence++}`;
        setRole(this.element, "listbox");
        if (options.ariaLabel) {
            setAriaAttribute(this.element, "label", options.ariaLabel);
        }
        this.defer(() => this.element.remove());
        this.own(addDisposableListener(this.element, "mousemove", (event) => {
            const index = this.#rowIndexFromEvent(event);
            if (index !== undefined)
                this.setActiveIndex(index);
        }));
        this.own(addDisposableListener(this.element, "mousedown", (event) => {
            if (this.#rowIndexFromEvent(event) !== undefined) {
                stopEvent(event);
            }
        }));
        this.own(addDisposableListener(this.element, "click", (event) => {
            const index = this.#rowIndexFromEvent(event);
            if (index === undefined)
                return;
            this.setActiveIndex(index);
            this.acceptActive(event);
        }));
    }
    get items() {
        return this.#items;
    }
    set items(items) {
        this.#items = [...items];
        const rows = this.#items.map((item, index) => {
            const row = this.element.ownerDocument.createElement("div");
            row.className = "zeta-list-row";
            row.id = `${this.element.id}-item-${index}`;
            row.dataset.index = String(index);
            setRole(row, "option");
            setAriaAttribute(row, "selected", false);
            row.append(this.#renderItem(item, index));
            return row;
        });
        this.element.replaceChildren(...rows);
        this.#activeIndex = rows.length > 0 ? 0 : -1;
        this.#syncActiveRows();
    }
    get activeIndex() {
        return this.#activeIndex;
    }
    get activeItem() {
        return this.#items[this.#activeIndex];
    }
    setActiveIndex(index) {
        if (!Number.isInteger(index) || index < 0 || index >= this.#items.length) {
            return;
        }
        if (this.#activeIndex === index)
            return;
        this.#activeIndex = index;
        this.#syncActiveRows();
    }
    focusNext() {
        this.#moveActive(1);
    }
    focusPrevious() {
        this.#moveActive(-1);
    }
    acceptActive(browserEvent) {
        const item = this.activeItem;
        if (item === undefined)
            return;
        this.#onDidAccept.fire({
            item,
            index: this.#activeIndex,
            browserEvent,
        });
    }
    #moveActive(delta) {
        const length = this.#items.length;
        if (length === 0)
            return;
        const candidate = this.#activeIndex + delta;
        const next = this.#loopNavigation
            ? (candidate + length) % length
            : Math.max(0, Math.min(candidate, length - 1));
        this.setActiveIndex(next);
    }
    #syncActiveRows() {
        const rows = this.element.querySelectorAll(":scope > .zeta-list-row");
        rows.forEach((row, index) => {
            const active = index === this.#activeIndex;
            row.classList.toggle("is-active", active);
            setAriaAttribute(row, "selected", active);
            if (active)
                row.scrollIntoView?.({ block: "nearest" });
        });
        const activeRow = rows[this.#activeIndex];
        this.#onDidChangeActive.fire({
            item: this.activeItem,
            index: this.#activeIndex,
            rowId: activeRow?.id,
        });
    }
    #rowIndexFromEvent(event) {
        if (!isHTMLElement(event.target))
            return undefined;
        const row = event.target.closest(".zeta-list-row");
        if (!row || row.parentElement !== this.element)
            return undefined;
        const index = Number(row.dataset.index);
        return Number.isInteger(index) ? index : undefined;
    }
}
let listSequence = 1;
