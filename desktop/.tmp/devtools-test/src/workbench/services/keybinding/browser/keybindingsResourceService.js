import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner, toDisposable, } from "../../../../base/common/lifecycle.js";
import { validateKeybindingsResource, validateKeybindingsResourceSnapshot, } from "../../../../platform/keybinding/common/keybindingsResource.js";
/**
 * Window projection of the active host-authoritative `keybindings.json`.
 */
export class WorkbenchKeybindingsResourceService extends DisposableOwner {
    #api;
    #onError;
    #onDidChangeKeybindings = this.own(new Emitter());
    #revision = 0;
    #bindings = [];
    #hasAuthoritativeSnapshot;
    #initialLoad;
    onDidChangeKeybindings = this.#onDidChangeKeybindings.event;
    constructor(options = {}) {
        super();
        this.#api = options.api;
        this.#onError = options.onError ??
            ((error) => console.error("Failed to apply keybindings resource", error));
        this.#hasAuthoritativeSnapshot = this.#api === undefined;
        if (this.#api) {
            const subscription = this.#api.onDidChange((candidate) => {
                try {
                    this.#acceptSnapshot(validateKeybindingsResourceSnapshot(candidate));
                }
                catch (error) {
                    this.#onError(error);
                }
            });
            this.own(toDisposable(() => subscription.dispose()));
        }
    }
    getKeybindings() {
        return this.#bindings;
    }
    async updateKeybindings(candidate) {
        const bindings = validateKeybindingsResource(candidate);
        if (this.#api && !this.#hasAuthoritativeSnapshot) {
            await this.reload();
        }
        if (!this.#api) {
            this.#acceptSnapshot({
                revision: this.#revision + 1,
                bindings,
            });
            return;
        }
        const result = await this.#api.update({
            expectedRevision: this.#revision,
            bindings,
        });
        this.#acceptSnapshot(validateKeybindingsResourceSnapshot(result));
    }
    async reload() {
        if (!this.#api)
            return;
        if (!this.#initialLoad) {
            this.#initialLoad = this.#api.read()
                .then((candidate) => {
                this.#acceptSnapshot(validateKeybindingsResourceSnapshot(candidate));
            })
                .finally(() => {
                this.#initialLoad = undefined;
            });
        }
        await this.#initialLoad;
    }
    #acceptSnapshot(snapshot) {
        if (!this.#hasAuthoritativeSnapshot) {
            this.#hasAuthoritativeSnapshot = true;
            this.#applySnapshot(snapshot);
            return;
        }
        if (snapshot.revision < this.#revision)
            return;
        const serialized = JSON.stringify(snapshot.bindings);
        if (snapshot.revision === this.#revision &&
            serialized === JSON.stringify(this.#bindings)) {
            return;
        }
        if (snapshot.revision === this.#revision) {
            throw new Error("Keybindings resource changed without advancing its revision");
        }
        this.#applySnapshot(snapshot);
    }
    #applySnapshot(snapshot) {
        if (JSON.stringify(snapshot.bindings) === JSON.stringify(this.#bindings)) {
            this.#revision = snapshot.revision;
            this.#bindings = snapshot.bindings;
            return;
        }
        this.#revision = snapshot.revision;
        this.#bindings = snapshot.bindings;
        this.#onDidChangeKeybindings.fire(this.#bindings);
    }
}
