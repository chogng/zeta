function defaultResourceKey(resource) {
    return resource.toString();
}
/**
 * A map keyed by URI value instead of URI object identity.
 *
 * Exact serialized URI identity is used by default. Pass an `IExtUri`
 * comparison-key method when provider-specific casing or fragment handling is
 * required.
 */
export class ResourceMap {
    #entries = new Map();
    #toKey;
    [Symbol.toStringTag] = "ResourceMap";
    constructor(toKey = defaultResourceKey) {
        this.#toKey = toKey;
    }
    get size() {
        return this.#entries.size;
    }
    has(resource) {
        return this.#entries.has(this.#toKey(resource));
    }
    get(resource) {
        return this.#entries.get(this.#toKey(resource))?.value;
    }
    set(resource, value) {
        this.#entries.set(this.#toKey(resource), { resource, value });
        return this;
    }
    delete(resource) {
        return this.#entries.delete(this.#toKey(resource));
    }
    clear() {
        this.#entries.clear();
    }
    forEach(callback, thisArg) {
        for (const entry of this.#entries.values()) {
            callback.call(thisArg, entry.value, entry.resource, this);
        }
    }
    *keys() {
        for (const entry of this.#entries.values()) {
            yield entry.resource;
        }
    }
    *values() {
        for (const entry of this.#entries.values()) {
            yield entry.value;
        }
    }
    *entries() {
        for (const entry of this.#entries.values()) {
            yield [entry.resource, entry.value];
        }
    }
    [Symbol.iterator]() {
        return this.entries();
    }
}
/** A set using the same URI key semantics as `ResourceMap`. */
export class ResourceSet {
    #resources;
    [Symbol.toStringTag] = "ResourceSet";
    constructor(toKey = defaultResourceKey) {
        this.#resources = new ResourceMap(toKey);
    }
    get size() {
        return this.#resources.size;
    }
    has(resource) {
        return this.#resources.has(resource);
    }
    add(resource) {
        this.#resources.set(resource, resource);
        return this;
    }
    delete(resource) {
        return this.#resources.delete(resource);
    }
    clear() {
        this.#resources.clear();
    }
    forEach(callback, thisArg) {
        this.#resources.forEach((_value, resource) => {
            callback.call(thisArg, resource, resource, this);
        });
    }
    entries() {
        return this.#resources.entries();
    }
    keys() {
        return this.#resources.keys();
    }
    values() {
        return this.#resources.keys();
    }
    [Symbol.iterator]() {
        return this.values();
    }
}
