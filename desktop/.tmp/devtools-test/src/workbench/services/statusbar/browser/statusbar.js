import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner, } from "../../../../base/common/lifecycle.js";
import { createServiceIdentifier, } from "../../../../platform/instantiation/common/instantiation.js";
/** The side of the status bar that owns an entry. */
export var StatusbarAlignment;
(function (StatusbarAlignment) {
    StatusbarAlignment["Left"] = "left";
    StatusbarAlignment["Right"] = "right";
})(StatusbarAlignment || (StatusbarAlignment = {}));
export const IStatusbarService = createServiceIdentifier("statusbarService");
/** Default window-scoped status bar entry service. */
export class StatusbarService extends DisposableOwner {
    #onDidChangeEntries = this.own(new Emitter());
    #entries = new Map();
    #nextOrder = 0;
    #disposed = false;
    onDidChangeEntries = this.#onDidChangeEntries.event;
    constructor() {
        super();
        this.defer(() => {
            this.#disposed = true;
            this.#entries.clear();
        });
    }
    addEntry(entry, options) {
        if (this.#disposed) {
            throw new ReferenceError("StatusbarService is already disposed");
        }
        if (!options.id) {
            throw new Error("A status bar entry requires a non-empty id");
        }
        if (this.#entries.has(options.id)) {
            throw new Error(`Status bar entry already exists: ${options.id}`);
        }
        const priority = options.priority ?? 0;
        if (!Number.isFinite(priority)) {
            throw new Error("Status bar entry priority must be finite");
        }
        let stored = {
            id: options.id,
            alignment: options.alignment,
            priority,
            entry: { ...entry },
            order: this.#nextOrder++,
        };
        this.#entries.set(stored.id, stored);
        this.#onDidChangeEntries.fire();
        return new StatusbarEntryAccessor((nextEntry) => {
            if (this.#disposed || this.#entries.get(stored.id) !== stored)
                return;
            stored = {
                ...stored,
                entry: { ...nextEntry },
            };
            this.#entries.set(stored.id, stored);
            this.#onDidChangeEntries.fire();
        }, () => {
            if (this.#disposed || this.#entries.get(stored.id) !== stored)
                return;
            this.#entries.delete(stored.id);
            this.#onDidChangeEntries.fire();
        });
    }
    getEntries(alignment) {
        return [...this.#entries.values()]
            .filter((item) => item.alignment === alignment)
            .sort(compareEntries)
            .map(({ id, entry, priority, alignment: itemAlignment }) => ({
            id,
            entry,
            priority,
            alignment: itemAlignment,
        }));
    }
}
class StatusbarEntryAccessor extends DisposableOwner {
    #update;
    #active = true;
    constructor(update, remove) {
        super();
        this.#update = update;
        this.defer(() => {
            this.#active = false;
            remove();
        });
    }
    update(entry) {
        if (!this.#active)
            return;
        this.#update(entry);
    }
}
function compareEntries(first, second) {
    return second.priority - first.priority || first.order - second.order;
}
