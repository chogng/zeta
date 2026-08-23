import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type LanguageWorkerWirePort } from "../../common/languages/languageWorkerWire.js";

interface DedicatedWorkerScope {
	postMessage(message: unknown): void;
	addEventListener(type: "message", listener: (event: { readonly data: unknown }) => void): void;
	removeEventListener(type: "message", listener: (event: { readonly data: unknown }) => void): void;
}

/** Adapts the current dedicated Worker global scope without owning it. */
export class DedicatedWorkerLanguagePort extends DisposableOwner implements LanguageWorkerWirePort {
	private readonly messageEmitter = this.own(new Emitter<unknown>());
	private disposed = false;

	readonly onMessage: Event<unknown> = this.messageEmitter.event;

	constructor(private readonly scope: DedicatedWorkerScope) {
		super();
		const onMessage = (event: { readonly data: unknown }): void => this.messageEmitter.fire(event.data);
		scope.addEventListener("message", onMessage);
		this.defer(() => {
			this.disposed = true;
			scope.removeEventListener("message", onMessage);
		});
	}

	send(message: unknown): void {
		if (this.disposed) {
			throw new ReferenceError("DedicatedWorkerLanguagePort is already disposed");
		}
		this.scope.postMessage(message);
	}
}

export function createDedicatedWorkerLanguagePort(): DedicatedWorkerLanguagePort {
	return new DedicatedWorkerLanguagePort(globalThis as unknown as DedicatedWorkerScope);
}
