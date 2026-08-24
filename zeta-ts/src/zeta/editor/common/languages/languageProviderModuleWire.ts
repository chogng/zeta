import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { assertLanguageProviderModuleId, assertLanguageProviderModuleState, LanguageProviderModuleHost, LanguageProviderModuleRegistry, LanguageProviderModuleState, normalizeLanguageProviderModuleCatalog, type LanguageProviderModuleCatalog, type LanguageProviderModuleController, type LanguageProviderModuleStateChange } from "./languageProviderModules.js";
import { type LanguageWorkerWirePort } from "./languageWorkerWire.js";

export interface LanguageProviderModuleWireDescriptor {
	readonly protocol: string;
	readonly version: number;
}

/** Renderer-side controller for one named Worker provider-module channel. */
export class LanguageProviderModuleWireClient extends DisposableOwner implements LanguageProviderModuleController {
	private readonly catalogEmitter = this.own(new Emitter<LanguageProviderModuleCatalog>());
	private readonly pending = new Map<number, PendingModuleRequest>();
	private readonly catalogWaiters = new Set<ModuleCatalogWaiter>();
	private catalog: LanguageProviderModuleCatalog = EMPTY_MODULE_CATALOG;
	private nextRequestId = 1;
	private catalogReady = false;
	private failure: Error | undefined;

	readonly onDidChangeModuleCatalog: Event<LanguageProviderModuleCatalog> = this.catalogEmitter.event;

	constructor(
		private readonly port: LanguageWorkerWirePort,
		private readonly descriptor: LanguageProviderModuleWireDescriptor,
		private readonly invalidateWorker: (error: Error) => void,
	) {
		super();
		assertPort(port);
		assertDescriptor(descriptor);
		if (typeof invalidateWorker !== "function") throw new TypeError("Provider module wire client requires an invalidation callback");
		this.own(port.onMessage(message => this.receive(message)));
		this.defer(() => {
			this.catalog = EMPTY_MODULE_CATALOG;
			this.catalogReady = false;
			this.failPending(new ReferenceError("LanguageProviderModuleWireClient is already disposed"));
			this.catalogEmitter.fire(this.catalog);
		});
	}

	get moduleCatalog(): LanguageProviderModuleCatalog {
		this.ensureAlive();
		return this.catalog;
	}

	get moduleCatalogReady(): boolean {
		return this.catalogReady;
	}

	waitForModuleCatalog(): Promise<LanguageProviderModuleCatalog> {
		try {
			this.ensureAlive();
		} catch (error) {
			return Promise.reject(error);
		}
		if (this.catalogReady) return Promise.resolve(this.catalog);
		return new Promise((resolve, reject) => {
			this.catalogWaiters.add({ resolve, reject });
		});
	}

	async setProviderModuleActivation(moduleId: string, state: LanguageProviderModuleState): Promise<LanguageProviderModuleStateChange> {
		this.ensureAlive();
		assertLanguageProviderModuleId(moduleId);
		assertLanguageProviderModuleState(state);
		const catalog = await this.waitForModuleCatalog();
		this.ensureAlive();
		if (state === LanguageProviderModuleState.Active && !catalog.modules.some(module => module.id === moduleId)) {
			throw new ReferenceError(`Language provider module '${moduleId}' is unavailable`);
		}
		const requestId = this.nextRequestId++;
		return new Promise((resolve, reject) => {
			this.pending.set(requestId, { moduleId, state, resolve, reject });
			try {
				this.port.send(Object.freeze({
					protocol: this.descriptor.protocol,
					version: this.descriptor.version,
					kind: "setActivation",
					requestId,
					moduleId,
					state,
				}));
			} catch (error) {
				this.pending.delete(requestId);
				reject(error);
			}
		});
	}

	invalidate(error: Error): void {
		this.fail(error);
	}

	private receive(value: unknown): void {
		if (!isModuleMessage(value, this.descriptor)) return;
		try {
			assertEnvelope(value, this.descriptor);
			if (value.kind === "catalog") {
				this.acceptCatalog(value.catalog);
				return;
			}
			const requestId = readRequestId(value.requestId);
			const pending = this.pending.get(requestId);
			if (!pending) return;
			this.pending.delete(requestId);
			if (value.kind === "failure") {
				pending.reject(decodeRemoteError(value.error));
				return;
			}
			if (value.kind !== "activation") throw new TypeError(`Unknown provider module response '${String(value.kind)}'`);
			if (value.moduleId !== pending.moduleId || value.state !== pending.state || typeof value.changed !== "boolean") {
				throw new Error("Provider module activation response does not match its request");
			}
			pending.resolve(Object.freeze({ moduleId: pending.moduleId, state: pending.state, changed: value.changed }));
		} catch (error) {
			this.fail(asError(error));
		}
	}

	private acceptCatalog(value: unknown): void {
		const catalog = normalizeLanguageProviderModuleCatalog(value);
		if (this.catalogReady && catalog.revision <= this.catalog.revision) {
			throw new Error("Provider module catalog revision must increase");
		}
		this.catalog = catalog;
		this.catalogReady = true;
		const waiters = [...this.catalogWaiters];
		this.catalogWaiters.clear();
		for (const waiter of waiters) waiter.resolve(catalog);
		this.catalogEmitter.fire(catalog);
	}

	private fail(error: Error): void {
		if (this.failure) return;
		this.failure = error;
		this.catalog = EMPTY_MODULE_CATALOG;
		this.catalogReady = false;
		this.failPending(error);
		this.catalogEmitter.fire(this.catalog);
		try {
			this.invalidateWorker(error);
		} catch {
			// The original protocol failure remains authoritative.
		}
	}

	private failPending(error: Error): void {
		const pending = [...this.pending.values()];
		this.pending.clear();
		for (const request of pending) request.reject(error);
		const waiters = [...this.catalogWaiters];
		this.catalogWaiters.clear();
		for (const waiter of waiters) waiter.reject(error);
	}

	private ensureAlive(): void {
		this.assertNotDisposed();
		if (this.failure) throw this.failure;
	}
}

/** Worker-side activation dispatcher and module-catalog publisher. */
export class LanguageProviderModuleWireServer<TProvider> extends DisposableOwner {

	constructor(
		private readonly port: LanguageWorkerWirePort,
		private readonly descriptor: LanguageProviderModuleWireDescriptor,
		modules: LanguageProviderModuleRegistry<TProvider>,
		private readonly host: LanguageProviderModuleHost<TProvider>,
	) {
		super();
		assertPort(port);
		assertDescriptor(descriptor);
		if (!(modules instanceof LanguageProviderModuleRegistry) || !(host instanceof LanguageProviderModuleHost)) {
			throw new TypeError("Provider module wire server requires its registry and host");
		}
		this.own(port.onMessage(message => this.receive(message)));
		this.own(modules.onDidChangeModuleCatalog(catalog => this.publishCatalog(catalog)));
		try {
			this.publishCatalog(modules.moduleCatalog);
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private receive(value: unknown): void {
		if (!isModuleMessage(value, this.descriptor) || this.isDisposed) return;
		let request: ActivationRequestMessage;
		try {
			assertEnvelope(value, this.descriptor);
			if (value.kind !== "setActivation") throw new TypeError(`Unknown provider module request '${String(value.kind)}'`);
			request = decodeActivationRequest(value);
		} catch (error) {
			const requestId = tryReadRequestId(value.requestId);
			if (requestId !== undefined) this.sendFailure(requestId, error);
			return;
		}
		void this.apply(request);
	}

	private async apply(request: ActivationRequestMessage): Promise<void> {
		try {
			const result = await this.host.setActivation(request.moduleId, request.state);
			if (!this.isDisposed) {
				this.port.send(Object.freeze({
					protocol: this.descriptor.protocol,
					version: this.descriptor.version,
					kind: "activation",
					requestId: request.requestId,
					...result,
				}));
			}
		} catch (error) {
			if (!this.isDisposed) this.sendFailure(request.requestId, error);
		}
	}

	private publishCatalog(catalog: LanguageProviderModuleCatalog): void {
		if (this.isDisposed) return;
		this.port.send(Object.freeze({
			protocol: this.descriptor.protocol,
			version: this.descriptor.version,
			kind: "catalog",
			catalog,
		}));
	}

	private sendFailure(requestId: number, error: unknown): void {
		const normalized = asError(error);
		this.port.send(Object.freeze({
			protocol: this.descriptor.protocol,
			version: this.descriptor.version,
			kind: "failure",
			requestId,
			error: Object.freeze({ name: normalized.name, message: normalized.message }),
		}));
	}
}

interface PendingModuleRequest {
	readonly moduleId: string;
	readonly state: LanguageProviderModuleState;
	readonly resolve: (result: LanguageProviderModuleStateChange) => void;
	readonly reject: (error: unknown) => void;
}

interface ModuleCatalogWaiter {
	readonly resolve: (catalog: LanguageProviderModuleCatalog) => void;
	readonly reject: (error: Error) => void;
}

interface ActivationRequestMessage {
	readonly requestId: number;
	readonly moduleId: string;
	readonly state: LanguageProviderModuleState;
}

function decodeActivationRequest(value: Record<string, unknown>): ActivationRequestMessage {
	const requestId = readRequestId(value.requestId);
	assertLanguageProviderModuleId(value.moduleId);
	assertLanguageProviderModuleState(value.state);
	return { requestId, moduleId: value.moduleId, state: value.state };
}

function isModuleMessage(value: unknown, descriptor: LanguageProviderModuleWireDescriptor): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && (value as Record<string, unknown>).protocol === descriptor.protocol;
}

function assertEnvelope(value: Record<string, unknown>, descriptor: LanguageProviderModuleWireDescriptor): void {
	if (value.version !== descriptor.version) {
		throw new RangeError(`Unsupported provider module protocol version '${String(value.version)}'`);
	}
}

function decodeRemoteError(value: unknown): Error {
	if (typeof value !== "object" || value === null) throw new TypeError("Provider module failure must be an object");
	const error = value as { readonly name?: unknown; readonly message?: unknown };
	if (typeof error.name !== "string" || typeof error.message !== "string") {
		throw new TypeError("Provider module failure must contain name and message");
	}
	const result = new Error(error.message);
	result.name = error.name;
	return result;
}

function readRequestId(value: unknown): number {
	const result = tryReadRequestId(value);
	if (result === undefined) throw new RangeError("Provider module request ID must be a positive safe integer");
	return result;
}

function tryReadRequestId(value: unknown): number | undefined {
	return Number.isSafeInteger(value) && (value as number) > 0 ? value as number : undefined;
}

function assertPort(port: LanguageWorkerWirePort): void {
	if (!port || typeof port.send !== "function" || typeof port.onMessage !== "function") {
		throw new TypeError("Provider module wire port is invalid");
	}
}

function assertDescriptor(descriptor: LanguageProviderModuleWireDescriptor): void {
	if (!descriptor || typeof descriptor.protocol !== "string" || descriptor.protocol.length === 0 || !Number.isSafeInteger(descriptor.version) || descriptor.version < 1) {
		throw new TypeError("Provider module wire descriptor is invalid");
	}
}

function asError(value: unknown): Error {
	return value instanceof Error ? value : new Error(String(value));
}

const EMPTY_MODULE_CATALOG: LanguageProviderModuleCatalog = Object.freeze({
	revision: 0,
	modules: Object.freeze([]),
});
