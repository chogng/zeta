import { toDisposable, } from "../../../../base/common/lifecycle.js";
import { EditorPaneMatch, } from "./editorPane.js";
/** Owns the editor implementations available in one product module graph. */
export class EditorPaneRegistry {
    #descriptors = new Map();
    register(descriptor) {
        this.#add(descriptor);
        return toDisposable(() => {
            if (this.#descriptors.get(descriptor.id) === descriptor) {
                this.#descriptors.delete(descriptor.id);
            }
        });
    }
    /** Registers a descriptor that intentionally lives for the module realm. */
    registerStatic(descriptor) {
        this.#add(descriptor);
    }
    get(id) {
        return this.#descriptors.get(id);
    }
    /**
     * Returns compatible editors in default-selection order.
     *
     * Higher matches come first. Registration order resolves equal matches so
     * product contribution order remains deterministic.
     */
    getEditors(input) {
        return Array.from(this.#descriptors.values())
            .map((descriptor, index) => {
            const match = descriptor.canOpen(input);
            validateMatch(match, descriptor.id);
            return { descriptor, index, match };
        })
            .filter(({ match }) => match !== EditorPaneMatch.None)
            .sort((left, right) => right.match - left.match || left.index - right.index)
            .map(({ descriptor }) => descriptor);
    }
    resolve(input, options = {}) {
        const preferredEditorId = options.preferredEditorId;
        if (preferredEditorId !== undefined) {
            const preferred = this.#descriptors.get(preferredEditorId);
            if (!preferred) {
                throw new RangeError(`Unknown editor pane '${preferredEditorId}'`);
            }
            if (preferred.canOpen(input) === EditorPaneMatch.None) {
                throw new RangeError(`Editor pane '${preferredEditorId}' cannot open ${input.resource}`);
            }
            return preferred;
        }
        const selected = this.getEditors(input)[0];
        if (!selected) {
            throw new RangeError(`No editor can open ${input.resource}`);
        }
        return selected;
    }
    #add(descriptor) {
        validateDescriptor(descriptor);
        if (this.#descriptors.has(descriptor.id)) {
            throw new Error(`Editor pane is already registered: ${descriptor.id}`);
        }
        this.#descriptors.set(descriptor.id, descriptor);
    }
}
/** Realm-scoped declarations populated by the selected product entry. */
export const EditorPanes = new EditorPaneRegistry();
export function registerEditorPane(descriptor) {
    EditorPanes.registerStatic(descriptor);
}
function validateDescriptor(descriptor) {
    if (!/^[A-Za-z][A-Za-z0-9._-]{0,127}$/.test(descriptor.id)) {
        throw new TypeError(`Invalid editor pane ID: ${descriptor.id}`);
    }
    if (descriptor.name.trim().length === 0) {
        throw new TypeError(`Editor pane '${descriptor.id}' requires a name`);
    }
}
function validateMatch(match, editorId) {
    if (match !== EditorPaneMatch.None &&
        match !== EditorPaneMatch.Optional &&
        match !== EditorPaneMatch.Default) {
        throw new TypeError(`Editor pane '${editorId}' returned an invalid match`);
    }
}
