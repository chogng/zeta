import { List, } from "../../../base/browser/ui/list/list.js";
import { setRole } from "../../../base/browser/ui/aria/aria.js";
import { Emitter } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
/** Searchable single-selection list shared by browser Quick Inputs. */
export class QuickInputList extends DisposableOwner {
    element;
    #empty;
    #list;
    #onDidAccept = this.own(new Emitter());
    #onDidChangeActive = this.own(new Emitter());
    #items = [];
    #visibleItems = [];
    #query = "";
    onDidAccept = this.#onDidAccept.event;
    onDidChangeActive = this.#onDidChangeActive.event;
    constructor(ownerDocument) {
        super();
        this.element = ownerDocument.createElement("div");
        this.element.className = "zeta-quick-pick-list";
        this.defer(() => this.element.remove());
        this.#list = this.own(new List({
            ownerDocument,
            ariaLabel: "Quick Pick results",
            renderItem: (item) => this.#renderItem(item),
        }));
        this.#list.element.classList.add("zeta-quick-pick-list-items");
        this.#empty = ownerDocument.createElement("div");
        this.#empty.className = "zeta-quick-pick-empty";
        setRole(this.#empty, "status");
        this.#empty.textContent = "No matching results";
        this.#empty.hidden = true;
        this.element.append(this.#list.element, this.#empty);
        this.own(this.#list.onDidAccept(({ item }) => {
            this.#onDidAccept.fire(item);
        }));
        this.own(this.#list.onDidChangeActive(({ item, rowId }) => {
            this.#onDidChangeActive.fire({ item, rowId });
        }));
    }
    get listId() {
        return this.#list.element.id;
    }
    get items() {
        return this.#items;
    }
    set items(items) {
        this.#items = [...items];
        this.#render();
    }
    get visibleItems() {
        return this.#visibleItems;
    }
    get activeItem() {
        return this.#list.activeItem;
    }
    filter(query) {
        if (this.#query === query)
            return;
        this.#query = query;
        this.#render();
    }
    focusNext() {
        this.#list.focusNext();
    }
    focusPrevious() {
        this.#list.focusPrevious();
    }
    acceptActive() {
        this.#list.acceptActive();
    }
    #render() {
        this.#visibleItems = filterQuickPickItems(this.#items, this.#query);
        this.#list.items = this.#visibleItems;
        const empty = this.#visibleItems.length === 0;
        this.#list.element.hidden = empty;
        this.#empty.hidden = !empty;
    }
    #renderItem(item) {
        const ownerDocument = this.element.ownerDocument;
        const content = ownerDocument.createElement("div");
        content.className = "zeta-quick-pick-row-content";
        const text = ownerDocument.createElement("span");
        text.className = "zeta-quick-pick-row-text";
        const label = ownerDocument.createElement("span");
        label.className = "zeta-quick-pick-row-label";
        label.textContent = item.label;
        text.append(label);
        appendOptionalText(text, item.description, "zeta-quick-pick-row-description", ownerDocument);
        appendOptionalText(text, item.detail, "zeta-quick-pick-row-detail", ownerDocument);
        content.append(text);
        if (item.keybinding) {
            const keybinding = ownerDocument.createElement("kbd");
            keybinding.className = "zeta-quick-pick-row-keybinding";
            keybinding.textContent = item.keybinding;
            content.append(keybinding);
        }
        return content;
    }
}
export function filterQuickPickItems(items, query) {
    const tokens = normalize(query).split(/\s+/).filter(Boolean);
    if (tokens.length === 0)
        return [...items];
    return items
        .map((item, index) => ({
        item,
        index,
        score: scoreItem(item, tokens),
    }))
        .filter((entry) => entry.score >= 0)
        .sort((left, right) => right.score - left.score || left.index - right.index)
        .map((entry) => entry.item);
}
function scoreItem(item, tokens) {
    const label = normalize(item.label);
    const searchable = normalize([item.label, item.description, item.detail].filter(Boolean).join(" "));
    let score = 0;
    for (const token of tokens) {
        const labelScore = scoreSubsequence(label, token);
        const searchableScore = scoreSubsequence(searchable, token);
        const tokenScore = labelScore >= 0
            ? Math.max(labelScore + 40, searchableScore)
            : searchableScore;
        if (tokenScore < 0)
            return -1;
        score += tokenScore;
    }
    return score;
}
function scoreSubsequence(value, query) {
    let valueIndex = 0;
    let score = 0;
    let previousMatch = -2;
    for (const character of query) {
        const match = value.indexOf(character, valueIndex);
        if (match < 0)
            return -1;
        score += match === previousMatch + 1 ? 8 : 2;
        if (match === 0 || /[\s._:/-]/.test(value[match - 1] ?? "")) {
            score += 6;
        }
        previousMatch = match;
        valueIndex = match + 1;
    }
    return score - Math.max(0, value.length - query.length) / 100;
}
function appendOptionalText(container, value, className, ownerDocument) {
    if (!value)
        return;
    const element = ownerDocument.createElement("span");
    element.className = className;
    element.textContent = value;
    container.append(element);
}
function normalize(value) {
    return value.trim().toLocaleLowerCase("en-US");
}
