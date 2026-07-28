import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner, toDisposable, } from "../../../../base/common/lifecycle.js";
import { emptyConfigurationDocument, validateConfigurationDocument, validateConfigurationSnapshot, } from "../../../../platform/configuration/common/configuration.js";
import { ConfigurationsRegistry, } from "../../../../platform/configuration/common/configurationRegistry.js";
/**
 * Window-scoped projection of the host-authoritative configuration.
 *
 * Persisted values are validated through registered typed keys. Invalid
 * values fall back atomically to their defaults without mutating the source.
 */
export class WorkbenchConfigurationService extends DisposableOwner {
    #api;
    #registry;
    #onError;
    #onDidChangeConfiguration = this.own(new Emitter());
    #values = new Map();
    #revision = 0;
    #document = emptyConfigurationDocument();
    #hasAuthoritativeSnapshot;
    #initialLoad;
    onDidChangeConfiguration = this.#onDidChangeConfiguration.event;
    constructor(options = {}) {
        super();
        this.#api = options.api;
        this.#registry = options.registry ?? ConfigurationsRegistry;
        this.#onError = options.onError ??
            ((error) => console.error("Failed to apply configuration", error));
        this.#hasAuthoritativeSnapshot = this.#api === undefined;
        this.#rebuildValues();
        if (this.#api) {
            const subscription = this.#api.onDidChange((candidate) => {
                try {
                    this.#acceptSnapshot(validateConfigurationSnapshot(candidate));
                }
                catch (error) {
                    this.#onError(error);
                }
            });
            this.own(toDisposable(() => subscription.dispose()));
        }
    }
    getValue(key) {
        this.#assertRegistered(key);
        return this.#values.get(key);
    }
    async updateValue(key, value) {
        this.#assertRegistered(key);
        const serialized = key.serialize(value);
        key.parse(serialized);
        if (this.#api && !this.#hasAuthoritativeSnapshot) {
            await this.reload();
        }
        const document = validateConfigurationDocument({
            version: 1,
            values: {
                ...this.#document.values,
                [key.key]: serialized,
            },
        });
        await this.#writeDocument(document);
    }
    async resetValue(key) {
        this.#assertRegistered(key);
        if (this.#api && !this.#hasAuthoritativeSnapshot) {
            await this.reload();
        }
        const values = {
            ...this.#document.values,
        };
        delete values[key.key];
        await this.#writeDocument(validateConfigurationDocument({
            version: 1,
            values,
        }));
    }
    async #writeDocument(document) {
        if (!this.#api) {
            this.#acceptSnapshot({
                revision: this.#revision + 1,
                document,
            });
            return;
        }
        const result = await this.#api.update({
            expectedRevision: this.#revision,
            document,
        });
        this.#acceptSnapshot(validateConfigurationSnapshot(result));
    }
    async reload() {
        if (!this.#api)
            return;
        if (!this.#initialLoad) {
            this.#initialLoad = this.#api.read()
                .then((candidate) => {
                this.#acceptSnapshot(validateConfigurationSnapshot(candidate));
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
        if (snapshot.revision === this.#revision &&
            serializeDocument(snapshot.document) ===
                serializeDocument(this.#document)) {
            return;
        }
        if (snapshot.revision === this.#revision) {
            throw new Error("Configuration changed without advancing its revision");
        }
        this.#applySnapshot(snapshot);
    }
    #applySnapshot(snapshot) {
        const changedKeys = changedConfigurationKeys(this.#document, snapshot.document);
        this.#revision = snapshot.revision;
        this.#document = snapshot.document;
        this.#rebuildValues();
        if (changedKeys.size === 0)
            return;
        this.#onDidChangeConfiguration.fire({
            keys: changedKeys,
            affectsConfiguration(key) {
                return changedKeys.has(key.key);
            },
        });
    }
    #rebuildValues() {
        this.#values.clear();
        for (const key of this.#registry.getConfigurations()) {
            const candidate = this.#document.values[key.key];
            if (candidate === undefined) {
                this.#values.set(key, key.defaultValue);
                continue;
            }
            try {
                this.#values.set(key, key.parse(candidate));
            }
            catch (error) {
                this.#values.set(key, key.defaultValue);
                this.#onError(new Error(`Invalid configuration value for '${key.key}'`, {
                    cause: error,
                }));
            }
        }
    }
    #assertRegistered(key) {
        if (!this.#registry.owns(key)) {
            throw new Error(`Unknown configuration key: ${key.key}`);
        }
    }
}
function changedConfigurationKeys(previous, next) {
    const keys = new Set([
        ...Object.keys(previous.values),
        ...Object.keys(next.values),
    ]);
    const changed = new Set();
    for (const key of keys) {
        if (JSON.stringify(previous.values[key]) !==
            JSON.stringify(next.values[key])) {
            changed.add(key);
        }
    }
    return changed;
}
function serializeDocument(document) {
    return JSON.stringify(document);
}
