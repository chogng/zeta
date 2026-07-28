import { Dimension, } from "../../../../base/browser/geometry.js";
import { DisposableOwner, DisposableSlot, } from "../../../../base/common/lifecycle.js";
import { createServiceIdentifier, } from "../../../../platform/instantiation/common/instantiation.js";
import { WorkbenchPart } from "../../part.js";
import { EditorPaneVisibility, } from "./editorPane.js";
import { EditorPanes, } from "./editorRegistry.js";
export const IEditorPart = createServiceIdentifier("editorPart");
/** The central content region that hosts the active workbench editor or view. */
export class EditorPart extends WorkbenchPart {
    #activeSession = this.own(new DisposableSlot());
    #registry;
    #activeInput;
    #dimension = Dimension.Zero;
    #openSequence = 0;
    #pendingSession;
    get minimumWidth() { return 120; }
    get minimumHeight() { return 84; }
    constructor(ownerDocument, registry = EditorPanes) {
        super("editor", ownerDocument);
        this.#registry = registry;
        this.element.setAttribute("aria-label", "Editor");
        const ResizeObserverConstructor = ownerDocument.defaultView?.ResizeObserver;
        if (ResizeObserverConstructor) {
            const observer = new ResizeObserverConstructor(([entry]) => {
                if (!entry)
                    return;
                const borderBox = entry.borderBoxSize[0];
                this.layout(new Dimension(borderBox?.inlineSize ?? entry.contentRect.width, borderBox?.blockSize ?? entry.contentRect.height));
            });
            observer.observe(this.contentElement, { box: "border-box" });
            this.defer(() => observer.disconnect());
        }
        this.defer(() => this.#cancelPendingOpen());
    }
    get activeInput() {
        return this.#activeInput;
    }
    get activePane() {
        return this.#activeSession.value?.pane;
    }
    async openEditor(input, options = {}) {
        const sequence = ++this.#openSequence;
        this.#cancelPendingOpen();
        const descriptor = this.#registry.resolve(input, options);
        const pane = descriptor.create({
            ownerDocument: this.element.ownerDocument,
        });
        if (pane.id !== descriptor.id) {
            pane.dispose();
            throw new TypeError(`Editor pane factory '${descriptor.id}' created '${pane.id}'`);
        }
        const session = new EditorPaneSession(pane, this.element.ownerDocument);
        this.#pendingSession = session;
        this.contentElement.append(session.element);
        try {
            pane.create(session.element);
            pane.setVisible(EditorPaneVisibility.Hidden);
            await pane.setInput(input, session.signal);
        }
        catch (error) {
            if (this.#pendingSession === session) {
                this.#pendingSession = undefined;
            }
            session.dispose();
            if (sequence !== this.#openSequence) {
                throw new EditorOpenSupersededError(input);
            }
            throw error;
        }
        if (sequence !== this.#openSequence ||
            this.#pendingSession !== session) {
            session.dispose();
            throw new EditorOpenSupersededError(input);
        }
        this.#pendingSession = undefined;
        const previous = this.#activeSession.value;
        previous?.pane.setVisible(EditorPaneVisibility.Hidden);
        this.contentElement.replaceChildren(session.element);
        this.#activeSession.replace(session);
        this.#activeInput = input;
        pane.layout(this.#dimension);
        pane.setVisible(EditorPaneVisibility.Visible);
        return pane;
    }
    setContent(content) {
        this.#openSequence += 1;
        this.#cancelPendingOpen();
        this.#activeInput = undefined;
        this.#activeSession.value?.pane.setVisible(EditorPaneVisibility.Hidden);
        this.#activeSession.clear();
        this.contentElement.replaceChildren(content);
    }
    layout(dimension) {
        this.#dimension = new Dimension(dimension.width, dimension.height);
        this.activePane?.layout(this.#dimension);
    }
    focus() {
        this.activePane?.focus();
    }
    #cancelPendingOpen() {
        const pending = this.#pendingSession;
        this.#pendingSession = undefined;
        pending?.dispose();
    }
}
class EditorPaneSession extends DisposableOwner {
    pane;
    element;
    signal;
    constructor(pane, ownerDocument) {
        super();
        this.pane = pane;
        const AbortControllerConstructor = ownerDocument.defaultView?.AbortController ?? AbortController;
        const abortController = new AbortControllerConstructor();
        this.signal = abortController.signal;
        this.element = ownerDocument.createElement("div");
        this.element.className = "zeta-editor-pane-host";
        this.defer(() => this.element.remove());
        this.own(pane);
        this.defer(() => pane.clearInput());
        this.defer(() => pane.setVisible(EditorPaneVisibility.Hidden));
        this.defer(() => abortController.abort());
    }
}
export class EditorOpenSupersededError extends Error {
    input;
    constructor(input) {
        super(`Editor opening was superseded: ${input.resource}`);
        this.input = input;
        this.name = "EditorOpenSupersededError";
    }
}
