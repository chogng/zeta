/** Constructor whose existing instances may receive development-time method updates. */
export type HotReloadableConstructor = Function & {
  readonly prototype: object;
};

/** Result of registering one class with the development hot-reload runtime. */
export type HotReloadClassRegistration =
  | "disabled"
  | "registered"
  | "unchanged"
  | "patched"
  | "incompatible";

type HotReloadGlobal = typeof globalThis & {
  $zetaHotReload_registerClass?: (
    id: string,
    constructor: HotReloadableConstructor,
  ) => HotReloadClassRegistration;
};

let enabled = false;
const constructors = new Map<string, HotReloadableConstructor>();

/** Enables the Renderer-only development hot-reload runtime for this realm. */
export function enableHotReload(): void {
  if (enabled) return;
  enabled = true;
  (globalThis as HotReloadGlobal).$zetaHotReload_registerClass =
    registerHotReloadableClass;
}

/** Returns whether development hot reload is enabled in the current realm. */
export function isHotReloadEnabled(): boolean {
  return enabled;
}

/**
 * Registers a stable class identity and patches its existing prototype on a
 * later registration from Vite. Callers must fall back to a full reload when
 * this function returns `incompatible`.
 */
export function registerHotReloadableClass(
  id: string,
  constructor: HotReloadableConstructor,
): HotReloadClassRegistration {
  if (!enabled) return "disabled";
  if (!id.trim()) throw new TypeError("Hot-reload class ID must not be empty");
  if (
    typeof constructor !== "function" ||
    typeof constructor.prototype !== "object" ||
    constructor.prototype === null
  ) {
    throw new TypeError("Hot-reload class registration requires a constructor");
  }

  const current = constructors.get(id);
  if (!current) {
    constructors.set(id, constructor);
    return "registered";
  }
  if (current === constructor) return "unchanged";
  if (!canPatchPrototype(current.prototype, constructor.prototype)) {
    return "incompatible";
  }

  patchPrototype(current.prototype, constructor.prototype);
  console.debug(`[hot-reload] Patched '${id}'`);
  return "patched";
}

function canPatchPrototype(current: object, replacement: object): boolean {
  if (Object.getPrototypeOf(current) !== Object.getPrototypeOf(replacement)) {
    return false;
  }

  const replacementKeys = new Set(Reflect.ownKeys(replacement));
  for (const key of Reflect.ownKeys(current)) {
    if (key === "constructor" || replacementKeys.has(key)) continue;
    if (Object.getOwnPropertyDescriptor(current, key)?.configurable === false) {
      return false;
    }
  }
  for (const key of replacementKeys) {
    if (key === "constructor") continue;
    const currentDescriptor = Object.getOwnPropertyDescriptor(current, key);
    if (currentDescriptor?.configurable === false) return false;
  }
  return true;
}

function patchPrototype(current: object, replacement: object): void {
  const replacementKeys = new Set(Reflect.ownKeys(replacement));
  for (const key of Reflect.ownKeys(current)) {
    if (key !== "constructor" && !replacementKeys.has(key)) {
      Reflect.deleteProperty(current, key);
    }
  }
  for (const key of replacementKeys) {
    if (key === "constructor") continue;
    const descriptor = Object.getOwnPropertyDescriptor(replacement, key);
    if (descriptor) Object.defineProperty(current, key, descriptor);
  }
}
