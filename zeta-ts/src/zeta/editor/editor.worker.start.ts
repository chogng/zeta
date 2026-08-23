import { DisposableStore, toDisposable, type IDisposable } from "../base/common/lifecycle.js";

/** Structured-clone port owned by one Aster dedicated-worker runtime. */
export interface AsterWorkerPort extends IDisposable {
	postMessage(message: unknown, transfer?: readonly Transferable[]): void;
	onDidReceiveMessage(listener: (message: unknown) => void): IDisposable;
}

/** Context supplied to one Aster dedicated-worker bootstrap. */
export interface AsterWorkerContext {
	readonly port: AsterWorkerPort;
	readonly resources: DisposableStore;
}

let activeResources: DisposableStore | undefined;

/**
 * Starts one Aster dedicated-worker runtime over the canonical structured-clone port.
 *
 * The worker entrypoint stays separate from `editor.api.ts`, so programmatic model
 * consumers never initialize a worker-global transport.
 */
export function start(bootstrap: (context: AsterWorkerContext) => void, portFactory: () => AsterWorkerPort = createDedicatedWorkerPort): void {
	if (typeof bootstrap !== "function") throw new TypeError("Aster worker bootstrap must be a function");
	if (activeResources) throw new Error("Aster worker has already started");
	const resources = new DisposableStore();
	activeResources = resources;
	resources.defer(() => {
		if (activeResources === resources) activeResources = undefined;
	});
	try {
		const port = resources.add(portFactory());
		bootstrap({ port, resources });
	} catch (error) {
		resources.dispose();
		throw error;
	}
}

function createDedicatedWorkerPort(): AsterWorkerPort {
	const scope = globalThis as unknown as DedicatedWorkerScope;
	if (typeof scope.postMessage !== "function" || typeof scope.addEventListener !== "function" || typeof scope.removeEventListener !== "function") {
		throw new ReferenceError("Aster worker must run in a dedicated worker scope");
	}
	let disposed = false;
	const dispose = (): void => {
		if (disposed) return;
		disposed = true;
		scope.close();
	};
	return {
		postMessage(message, transfer = []) {
			if (disposed) throw new ReferenceError("Aster worker port is disposed");
			scope.postMessage(message, [...transfer]);
		},
		onDidReceiveMessage(listener) {
			if (disposed) throw new ReferenceError("Aster worker port is disposed");
			if (typeof listener !== "function") throw new TypeError("Aster worker message listener must be a function");
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
