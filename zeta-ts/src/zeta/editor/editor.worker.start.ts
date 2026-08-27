import { DisposableStore, toDisposable, type IDisposable } from "../base/common/lifecycle.js";

/** Structured-clone port owned by one Stanza dedicated-worker runtime. */
export interface StanzaWorkerPort extends IDisposable {
	postMessage(message: unknown, transfer?: readonly Transferable[]): void;
	onDidReceiveMessage(listener: (message: unknown) => void): IDisposable;
}

/** Context supplied to one Stanza dedicated-worker bootstrap. */
export interface StanzaWorkerContext {
	readonly port: StanzaWorkerPort;
	readonly resources: DisposableStore;
}

let activeResources: DisposableStore | undefined;

/**
 * Starts one Stanza dedicated-worker runtime over the canonical structured-clone port.
 *
 * The worker entrypoint stays separate from `editor.api.ts`, so programmatic model
 * consumers never initialize a worker-global transport.
 */
export function start(bootstrap: (context: StanzaWorkerContext) => void, portFactory: () => StanzaWorkerPort = createDedicatedWorkerPort): void {
	if (typeof bootstrap !== "function") throw new TypeError("Stanza worker bootstrap must be a function");
	if (activeResources) throw new Error("Stanza worker has already started");
	const resources = new DisposableStore();
	activeResources = resources;
	resources.add(toDisposable(() => {
		if (activeResources === resources) activeResources = undefined;
	}));
	try {
		const port = resources.add(portFactory());
		bootstrap({ port, resources });
	} catch (error) {
		resources.dispose();
		throw error;
	}
}

function createDedicatedWorkerPort(): StanzaWorkerPort {
	const scope = globalThis as unknown as DedicatedWorkerScope;
	if (typeof scope.postMessage !== "function" || typeof scope.addEventListener !== "function" || typeof scope.removeEventListener !== "function") {
		throw new ReferenceError("Stanza worker must run in a dedicated worker scope");
	}
	let disposed = false;
	const dispose = (): void => {
		if (disposed) return;
		disposed = true;
		scope.close();
	};
	return {
		postMessage(message, transfer = []) {
			if (disposed) throw new ReferenceError("Stanza worker port is disposed");
			scope.postMessage(message, [...transfer]);
		},
		onDidReceiveMessage(listener) {
			if (disposed) throw new ReferenceError("Stanza worker port is disposed");
			if (typeof listener !== "function") throw new TypeError("Stanza worker message listener must be a function");
			const handler = (event: MessageEvent<unknown>) => listener(event.data);
			scope.addEventListener("message", handler);
			return toDisposable(() => scope.removeEventListener("message", handler));
		},
		dispose,
		[Symbol.dispose]: dispose,
	};
}

interface DedicatedWorkerScope {
	postMessage(message: unknown, transfer: Transferable[]): void;
	addEventListener(type: "message", listener: (event: MessageEvent<unknown>) => void): void;
	removeEventListener(type: "message", listener: (event: MessageEvent<unknown>) => void): void;
	close(): void;
}
