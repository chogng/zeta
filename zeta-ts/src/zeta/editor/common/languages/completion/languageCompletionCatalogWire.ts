import { Emitter, type Event } from "../../../../base/common/event.js";
import { DeferredPromise } from "../../../../base/common/async.js";
import { raceCancellation } from "../../../../base/common/cancellation.js";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { LanguageCompletionProviderRegistry, normalizeLanguageCompletionProviderCatalog, type LanguageCompletionProviderCatalog, type LanguageCompletionProviderCatalogSource, type LanguageCompletionRequest } from "./languageCompletionProviders.js";
import { LanguageCompletionProviderModuleState, type LanguageCompletionProviderModuleCatalog, type LanguageCompletionProviderModuleController, type LanguageCompletionProviderModuleStateChange } from "./languageCompletionProviderModules.js";
import { LanguageCompletionProviderModuleWireClient } from "./languageCompletionProviderModuleWire.js";
import { activateRequiredLanguageProviderModules, normalizeRequiredLanguageProviderModules } from "../languageProviderModules.js";
import { LanguageCompletionResolveWireClient } from "./languageCompletionResolveWire.js";
import { type LanguageCompletionLane, type LanguageCompletionWorker } from "./languageCompletionService.js";
import { languageCompletionWireCodec } from "./languageCompletionWire.js";
import { type LanguageWorkerModelSynchronizer, type LanguageWorkerRequest } from "../languageRequestCoordinator.js";
import { LanguageWorkerWireClient, type LanguageWorkerWireClientPort, type LanguageWorkerWirePort } from "../languageWorkerWire.js";
import { type LanguageCompletionItemDetails, type LanguageCompletionItemResolver, type LanguageCompletionResolveRequest, type LanguageCompletionResult } from "./languageCompletions.js";
import { type TextModelChange } from "../../core/textChange.js";

const CATALOG_PROTOCOL = "zeta.language.completion-provider-catalog";
const CATALOG_PROTOCOL_VERSION = 1;

/** Completion worker client with a provider-catalog side channel. */
export class LanguageCompletionCatalogWorkerClient extends Disposable implements LanguageCompletionWorker, LanguageWorkerModelSynchronizer, LanguageCompletionProviderCatalogSource, LanguageCompletionProviderModuleController, LanguageCompletionItemResolver {
	private readonly catalogEmitter = this._register(new Emitter<LanguageCompletionProviderCatalog>());
	private readonly waiters = new Set<DeferredPromise<LanguageCompletionProviderCatalog>>();
	private readonly worker: LanguageWorkerWireClient<LanguageCompletionLane, LanguageCompletionRequest, LanguageCompletionResult>;
	private readonly modules: LanguageCompletionProviderModuleWireClient;
	private readonly resolver: LanguageCompletionResolveWireClient;
	private readonly moduleReadiness: Promise<void>;
	private catalog: LanguageCompletionProviderCatalog = EMPTY_CATALOG;
	private catalogReady = false;
	private modulesReady = false;
	private failure: Error | undefined;

	readonly onDidChangeProviderCatalog: Event<LanguageCompletionProviderCatalog> = this.catalogEmitter.event;
	readonly onDidChangeModuleCatalog: Event<LanguageCompletionProviderModuleCatalog>;

	constructor(port: LanguageWorkerWireClientPort, options: LanguageCompletionCatalogWorkerClientOptions = {}) {
		super();
		const requiredProviderModules = normalizeRequiredLanguageProviderModules(options.requiredProviderModules);
		this.worker = this._register(new LanguageWorkerWireClient(port, languageCompletionWireCodec));
		this.modules = this._register(new LanguageCompletionProviderModuleWireClient(port, error => this.worker.invalidate(error)));
		this.resolver = this._register(new LanguageCompletionResolveWireClient(port, error => this.worker.invalidate(error)));
		this.onDidChangeModuleCatalog = this.modules.onDidChangeModuleCatalog;
		this._register(port.onMessage(message => this.receive(message)));
		this._register(this.worker.onDidFail(error => {
			this.modules.invalidate(error);
			this.resolver.invalidate(error);
			this.fail(error);
		}));
		this.moduleReadiness = this.activateRequiredModules(requiredProviderModules);
		void this.moduleReadiness.catch(() => undefined);
		this._register(toDisposable(() => {
			this.clearCatalog();
			this.failWaiters(new ReferenceError("LanguageCompletionCatalogWorkerClient is already disposed"));
		}));
	}

	get providerCatalogReady(): boolean {
		return this.catalogReady && this.modulesReady;
	}

	get providerCatalog(): LanguageCompletionProviderCatalog {
		this.ensureAlive();
		return this.catalog;
	}

	waitForProviderCatalog(): Promise<LanguageCompletionProviderCatalog> {
		try {
			this.ensureAlive();
		} catch (error) {
			return Promise.reject(error);
		}
		if (this.failure) return Promise.reject(this.failure);
		const catalogWaiter = new DeferredPromise<LanguageCompletionProviderCatalog>();
		if (!this.catalogReady) this.waiters.add(catalogWaiter);
		const catalog = this.catalogReady ? Promise.resolve(this.catalog) : catalogWaiter.p;
		return Promise.all([catalog, this.moduleReadiness]).then(() => {
			this.ensureAlive();
			return this.catalog;
		});
	}

	async run(request: LanguageWorkerRequest<LanguageCompletionLane, LanguageCompletionRequest>, signal: AbortSignal): Promise<LanguageCompletionResult> {
		this.ensureAlive();
		await raceCancellation(this.moduleReadiness, signal, "Language completion provider module wait was cancelled");
		this.ensureAlive();
		return this.worker.run(request, signal);
	}

	synchronizeModel(change: TextModelChange): void {
		this.ensureAlive();
		this.worker.synchronizeModel(change);
	}

	get moduleCatalog(): LanguageCompletionProviderModuleCatalog {
		return this.modules.moduleCatalog;
	}

	get moduleCatalogReady(): boolean {
		return this.modules.moduleCatalogReady;
	}

	waitForModuleCatalog(): Promise<LanguageCompletionProviderModuleCatalog> {
		return this.modules.waitForModuleCatalog();
	}

	setProviderModuleActivation(moduleId: string, state: LanguageCompletionProviderModuleState): Promise<LanguageCompletionProviderModuleStateChange> {
		return this.modules.setProviderModuleActivation(moduleId, state);
	}

	async resolveCompletionItem(request: LanguageCompletionResolveRequest, signal: AbortSignal): Promise<LanguageCompletionItemDetails> {
		this.ensureAlive();
		await raceCancellation(this.moduleReadiness, signal, "Language completion provider module wait was cancelled");
		this.ensureAlive();
		return this.resolver.resolveCompletionItem(request, signal);
	}

	private receive(value: unknown): void {
		if (!isCatalogMessage(value)) return;
		try {
			if (value.version !== CATALOG_PROTOCOL_VERSION || value.kind !== "catalog") {
				throw new Error("Unsupported completion provider catalog message");
			}
			const catalog = normalizeLanguageCompletionProviderCatalog(value.catalog);
			if (this.catalogReady && catalog.revision <= this.catalog.revision) {
				throw new Error("Completion provider catalog revision must increase");
			}
			this.catalog = catalog;
			this.catalogReady = true;
			const waiters = [...this.waiters];
			this.waiters.clear();
			for (const waiter of waiters) void waiter.complete(catalog);
			this.catalogEmitter.fire(catalog);
		} catch (error) {
			this.worker.invalidate(error);
		}
	}

	private fail(error: Error): void {
		if (!this.failure) this.failure = error;
		this.clearCatalog();
		this.failWaiters(this.failure);
	}

	private async activateRequiredModules(moduleIds: readonly string[]): Promise<void> {
		if (moduleIds.length === 0) {
			this.modulesReady = true;
			return;
		}
		try {
			await activateRequiredLanguageProviderModules(this.modules, moduleIds);
			this.modulesReady = true;
		} catch (error) {
			try {
				this.worker.invalidate(error);
			} catch {
				// The required-module failure remains authoritative.
			}
			throw error;
		}
	}

	private clearCatalog(): void {
		if (!this.catalogReady && this.catalog.providers.length === 0) return;
		this.catalog = EMPTY_CATALOG;
		this.catalogReady = false;
		this.catalogEmitter.fire(this.catalog);
	}

	private failWaiters(error: Error): void {
		const waiters = [...this.waiters];
		this.waiters.clear();
		for (const waiter of waiters) void waiter.error(error);
	}

	private ensureAlive(): void {
		this.assertNotDisposed();
		if (this.failure) throw this.failure;
	}
}

export interface LanguageCompletionCatalogWorkerClientOptions {
	readonly requiredProviderModules?: readonly string[];
}

/** Publishes the actual Worker registry snapshot and every later revision. */
export class LanguageCompletionCatalogWirePublisher extends Disposable {
	constructor(port: LanguageWorkerWirePort, registry: LanguageCompletionProviderRegistry) {
		super();
		assertPort(port);
		if (!(registry instanceof LanguageCompletionProviderRegistry)) {
			throw new TypeError("Completion catalog publisher requires a provider registry");
		}
		const publish = (catalog: LanguageCompletionProviderCatalog): void => {
			port.send(Object.freeze({
				protocol: CATALOG_PROTOCOL,
				version: CATALOG_PROTOCOL_VERSION,
				kind: "catalog",
				catalog,
			}));
		};
		this._register(registry.onDidChangeProviderCatalog(publish));
		try {
			publish(registry.providerCatalog);
		} catch (error) {
			this.dispose();
			throw error;
		}
	}
}

const EMPTY_CATALOG: LanguageCompletionProviderCatalog = Object.freeze({
	revision: 0,
	providers: Object.freeze([]),
});

function isCatalogMessage(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && (value as Record<string, unknown>).protocol === CATALOG_PROTOCOL;
}

function assertPort(port: LanguageWorkerWirePort): void {
	if (!port || typeof port.send !== "function" || typeof port.onMessage !== "function") {
		throw new TypeError("Completion catalog publisher port is invalid");
	}
}
