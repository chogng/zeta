import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner, toDisposable, type IDisposable } from "../../../../base/common/lifecycle.js";

export interface LanguageProviderModule<TProvider> {
  readonly id: string;
  load(): readonly TProvider[] | PromiseLike<readonly TProvider[]>;
}

export interface LanguageProviderModuleMetadata {
  readonly id: string;
}

export interface LanguageProviderModuleCatalog {
  readonly revision: number;
  readonly modules: readonly LanguageProviderModuleMetadata[];
}

export interface LanguageProviderModuleCatalogSource {
  readonly moduleCatalog: LanguageProviderModuleCatalog;
  readonly moduleCatalogReady: boolean;
  readonly onDidChangeModuleCatalog: Event<LanguageProviderModuleCatalog>;
  waitForModuleCatalog(): Promise<LanguageProviderModuleCatalog>;
}

export interface LanguageProviderModuleController extends LanguageProviderModuleCatalogSource {
  setProviderModuleActivation(moduleId: string, state: LanguageProviderModuleState): Promise<LanguageProviderModuleStateChange>;
}

export enum LanguageProviderModuleState {
  Active = "active",
  Inactive = "inactive",
}

export interface LanguageProviderModuleStateChange {
  readonly moduleId: string;
  readonly state: LanguageProviderModuleState;
  readonly changed: boolean;
}

export interface LanguageProviderBatchRegistry<TProvider> {
  registerMany(providers: readonly TProvider[]): IDisposable;
}

interface RegisteredProviderModule<TProvider> {
  readonly id: string;
  load(): readonly TProvider[] | PromiseLike<readonly TProvider[]>;
}

/** Caller-owned named provider definitions available in one Worker realm. */
export class LanguageProviderModuleRegistry<TProvider> extends DisposableOwner {
  private readonly catalogEmitter = this.own(new Emitter<LanguageProviderModuleCatalog>());
  private readonly modules = new Map<string, RegisteredProviderModule<TProvider>>();
  private catalog: LanguageProviderModuleCatalog = EMPTY_MODULE_CATALOG;
  private disposed = false;

  readonly onDidChangeModuleCatalog: Event<LanguageProviderModuleCatalog> = this.catalogEmitter.event;

  constructor() {
    super();
    this.defer(() => {
      const changed = this.modules.size > 0;
      this.modules.clear();
      if (changed) this.updateCatalog();
      this.disposed = true;
    });
  }

  get moduleCatalog(): LanguageProviderModuleCatalog {
    this.ensureAlive();
    return this.catalog;
  }

  register(module: LanguageProviderModule<TProvider>): IDisposable {
    this.ensureAlive();
    const registered = normalizeModule(module);
    if (this.modules.has(registered.id)) {
      throw new RangeError(`Language provider module '${registered.id}' is already registered`);
    }
    this.modules.set(registered.id, registered);
    this.updateCatalog();
    return toDisposable(() => {
      if (this.modules.get(registered.id) === registered) {
        this.modules.delete(registered.id);
        this.updateCatalog();
      }
    });
  }

  getModule(moduleId: string): LanguageProviderModule<TProvider> | undefined {
    this.ensureAlive();
    assertLanguageProviderModuleId(moduleId);
    return this.modules.get(moduleId);
  }

  private updateCatalog(): void {
    this.catalog = Object.freeze({
      revision: this.catalog.revision + 1,
      modules: Object.freeze([...this.modules.keys()].map(id => Object.freeze({ id }))),
    });
    this.catalogEmitter.fire(this.catalog);
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("LanguageProviderModuleRegistry is already disposed");
  }
}

/** Owns serialized module activation and atomic provider-registration batches. */
export class LanguageProviderModuleHost<TProvider> extends DisposableOwner {
  private readonly active = new Map<string, IDisposable>();
  private readonly operationTails = new Map<string, Promise<void>>();
  private disposed = false;

  constructor(
    private readonly modules: LanguageProviderModuleRegistry<TProvider>,
    private readonly providers: LanguageProviderBatchRegistry<TProvider>,
  ) {
    super();
    this.own(modules.onDidChangeModuleCatalog(catalog => {
      const available = new Set(catalog.modules.map(module => module.id));
      for (const [moduleId, registration] of this.active) {
        if (!available.has(moduleId)) {
          this.active.delete(moduleId);
          registration.dispose();
        }
      }
    }));
    this.defer(() => {
      this.disposed = true;
      const registrations = [...this.active.values()];
      this.active.clear();
      for (let index = registrations.length - 1; index >= 0; index -= 1) registrations[index]!.dispose();
    });
  }

  setActivation(moduleId: string, state: LanguageProviderModuleState): Promise<LanguageProviderModuleStateChange> {
    this.ensureAlive();
    assertLanguageProviderModuleId(moduleId);
    assertLanguageProviderModuleState(state);
    const previous = this.operationTails.get(moduleId) ?? Promise.resolve();
    const operation = previous.then(() => this.applyActivation(moduleId, state));
    const tail = operation.then(() => undefined, () => undefined);
    this.operationTails.set(moduleId, tail);
    void tail.finally(() => {
      if (this.operationTails.get(moduleId) === tail) this.operationTails.delete(moduleId);
    });
    return operation;
  }

  private async applyActivation(moduleId: string, state: LanguageProviderModuleState): Promise<LanguageProviderModuleStateChange> {
    this.ensureAlive();
    if (state === LanguageProviderModuleState.Inactive) {
      const registration = this.active.get(moduleId);
      if (!registration) return moduleStateChange(moduleId, state, false);
      this.active.delete(moduleId);
      registration.dispose();
      return moduleStateChange(moduleId, state, true);
    }
    if (this.active.has(moduleId)) return moduleStateChange(moduleId, state, false);
    const module = this.modules.getModule(moduleId);
    if (!module) throw new ReferenceError(`Language provider module '${moduleId}' is unavailable`);
    const providers = await module.load();
    this.ensureAlive();
    if (this.modules.getModule(moduleId) !== module) {
      throw new ReferenceError(`Language provider module '${moduleId}' was removed while loading`);
    }
    if (!Array.isArray(providers) || providers.length === 0) {
      throw new TypeError(`Language provider module '${moduleId}' must load providers`);
    }
    const registration = this.providers.registerMany(providers);
    this.active.set(moduleId, registration);
    return moduleStateChange(moduleId, state, true);
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("LanguageProviderModuleHost is already disposed");
  }
}

export function normalizeLanguageProviderModuleCatalog(value: unknown): LanguageProviderModuleCatalog {
  if (typeof value !== "object" || value === null) {
    throw new TypeError("Language provider module catalog must be an object");
  }
  const catalog = value as Partial<LanguageProviderModuleCatalog>;
  if (!Number.isSafeInteger(catalog.revision) || catalog.revision! < 0) {
    throw new RangeError("Language provider module catalog revision must be a non-negative safe integer");
  }
  if (!Array.isArray(catalog.modules)) {
    throw new TypeError("Language provider module catalog must contain modules");
  }
  const identities = new Set<string>();
  const modules = catalog.modules.map(module => {
    if (typeof module !== "object" || module === null) {
      throw new TypeError("Language provider module metadata must be an object");
    }
    assertLanguageProviderModuleId(module.id);
    if (identities.has(module.id)) throw new RangeError(`Duplicate language provider module '${module.id}'`);
    identities.add(module.id);
    return Object.freeze({ id: module.id });
  });
  return Object.freeze({ revision: catalog.revision!, modules: Object.freeze(modules) });
}

export function assertLanguageProviderModuleId(value: unknown): asserts value is string {
  if (typeof value !== "string" || !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(value)) {
    throw new TypeError("Language provider module ID is invalid");
  }
}

export function assertLanguageProviderModuleState(value: unknown): asserts value is LanguageProviderModuleState {
  if (value !== LanguageProviderModuleState.Active && value !== LanguageProviderModuleState.Inactive) {
    throw new TypeError(`Unknown language provider module state '${String(value)}'`);
  }
}

export function normalizeRequiredLanguageProviderModules(value: readonly string[] | undefined): readonly string[] {
  if (value === undefined) return Object.freeze([]);
  if (!Array.isArray(value)) throw new TypeError("Required language provider modules must be an array");
  const result = [...value];
  for (const moduleId of result) assertLanguageProviderModuleId(moduleId);
  if (new Set(result).size !== result.length) {
    throw new RangeError("Required language provider modules must be unique");
  }
  return Object.freeze(result);
}

export async function activateRequiredLanguageProviderModules(controller: LanguageProviderModuleController, moduleIds: readonly string[]): Promise<void> {
  if (moduleIds.length === 0) return;
  await controller.waitForModuleCatalog();
  for (const moduleId of moduleIds) {
    await controller.setProviderModuleActivation(moduleId, LanguageProviderModuleState.Active);
  }
}

function normalizeModule<TProvider>(module: LanguageProviderModule<TProvider>): RegisteredProviderModule<TProvider> {
  if (typeof module !== "object" || module === null) throw new TypeError("Language provider module must be an object");
  assertLanguageProviderModuleId(module.id);
  if (typeof module.load !== "function") throw new TypeError("Language provider module must implement load");
  return Object.freeze({ id: module.id, load: module.load.bind(module) });
}

function moduleStateChange(moduleId: string, state: LanguageProviderModuleState, changed: boolean): LanguageProviderModuleStateChange {
  return Object.freeze({ moduleId, state, changed });
}

const EMPTY_MODULE_CATALOG: LanguageProviderModuleCatalog = Object.freeze({
  revision: 0,
  modules: Object.freeze([]),
});
