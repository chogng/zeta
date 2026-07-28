import { URI } from "./uri.js";

/** Converts a URI into the string used by a resource collection. */
export type ResourceMapKeyFn = (resource: URI) => string;

interface ResourceMapEntry<T> {
  readonly resource: URI;
  readonly value: T;
}

function defaultResourceKey(resource: URI): string {
  return resource.toString();
}

/**
 * A map keyed by URI value instead of URI object identity.
 *
 * Exact serialized URI identity is used by default. Pass an `IExtUri`
 * comparison-key method when provider-specific casing or fragment handling is
 * required.
 */
export class ResourceMap<T> implements Map<URI, T> {
  readonly #entries = new Map<string, ResourceMapEntry<T>>();
  readonly #toKey: ResourceMapKeyFn;

  readonly [Symbol.toStringTag] = "ResourceMap";

  constructor(toKey: ResourceMapKeyFn = defaultResourceKey) {
    this.#toKey = toKey;
  }

  get size(): number {
    return this.#entries.size;
  }

  has(resource: URI): boolean {
    return this.#entries.has(this.#toKey(resource));
  }

  get(resource: URI): T | undefined {
    return this.#entries.get(this.#toKey(resource))?.value;
  }

  set(resource: URI, value: T): this {
    this.#entries.set(this.#toKey(resource), { resource, value });
    return this;
  }

  delete(resource: URI): boolean {
    return this.#entries.delete(this.#toKey(resource));
  }

  clear(): void {
    this.#entries.clear();
  }

  forEach(
    callback: (value: T, key: URI, map: Map<URI, T>) => void,
    thisArg?: unknown,
  ): void {
    for (const entry of this.#entries.values()) {
      callback.call(thisArg, entry.value, entry.resource, this);
    }
  }

  *keys(): MapIterator<URI> {
    for (const entry of this.#entries.values()) {
      yield entry.resource;
    }
  }

  *values(): MapIterator<T> {
    for (const entry of this.#entries.values()) {
      yield entry.value;
    }
  }

  *entries(): MapIterator<[URI, T]> {
    for (const entry of this.#entries.values()) {
      yield [entry.resource, entry.value];
    }
  }

  [Symbol.iterator](): MapIterator<[URI, T]> {
    return this.entries();
  }
}

/** A set using the same URI key semantics as `ResourceMap`. */
export class ResourceSet implements Set<URI> {
  readonly #resources: ResourceMap<URI>;

  readonly [Symbol.toStringTag] = "ResourceSet";

  constructor(toKey: ResourceMapKeyFn = defaultResourceKey) {
    this.#resources = new ResourceMap(toKey);
  }

  get size(): number {
    return this.#resources.size;
  }

  has(resource: URI): boolean {
    return this.#resources.has(resource);
  }

  add(resource: URI): this {
    this.#resources.set(resource, resource);
    return this;
  }

  delete(resource: URI): boolean {
    return this.#resources.delete(resource);
  }

  clear(): void {
    this.#resources.clear();
  }

  forEach(
    callback: (value: URI, value2: URI, set: Set<URI>) => void,
    thisArg?: unknown,
  ): void {
    this.#resources.forEach((_value, resource) => {
      callback.call(thisArg, resource, resource, this);
    });
  }

  entries(): SetIterator<[URI, URI]> {
    return this.#resources.entries();
  }

  keys(): SetIterator<URI> {
    return this.#resources.keys();
  }

  values(): SetIterator<URI> {
    return this.#resources.keys();
  }

  [Symbol.iterator](): SetIterator<URI> {
    return this.values();
  }
}
