import { Emitter } from "../../base/common/event.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
/**
 * Base class for a persistent visual region in the browser workbench shell.
 *
 * Parts own their layout constraints. WorkbenchLayout decides topology and
 * delegates the resulting pixel dimensions through `layout`.
 */
export class WorkbenchPart extends DisposableOwner {
    element;
    titleElement;
    contentElement;
    #onDidChangeConstraints = this.own(new Emitter());
    onDidChangeConstraints = this.#onDidChangeConstraints.event;
    constructor(id, ownerDocument) {
        super();
        const element = ownerDocument.createElement("section");
        this.element = element;
        this.defer(() => element.remove());
        element.className = `zeta-workbench-part zeta-workbench-${id}`;
        element.dataset.part = id;
        this.titleElement = ownerDocument.createElement("div");
        this.titleElement.className = "zeta-workbench-part-title";
        this.contentElement = ownerDocument.createElement("div");
        this.contentElement.className = "zeta-workbench-part-content";
        element.append(this.titleElement, this.contentElement);
    }
    get minimumWidth() { return 0; }
    get maximumWidth() { return Number.POSITIVE_INFINITY; }
    get minimumHeight() { return 0; }
    get maximumHeight() { return Number.POSITIVE_INFINITY; }
    layout(_dimension) { }
    setVisible(visible) {
        this.element.hidden = !visible;
    }
    /** Notifies the runtime layout after a subclass changes its constraints. */
    notifyConstraintsChanged() {
        this.#onDidChangeConstraints.fire();
    }
}
