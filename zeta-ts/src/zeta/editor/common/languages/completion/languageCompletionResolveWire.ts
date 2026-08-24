import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { normalizeLanguageCompletionItemDetails, normalizeLanguageCompletionResolveRequest, type LanguageCompletionItemDetails, type LanguageCompletionItemResolver, type LanguageCompletionResolveRequest } from "./languageCompletions.js";
import { type LanguageWorkerWirePort } from "../languageWorkerWire.js";

const RESOLVE_PROTOCOL = "zeta.language.completion-resolve";
const RESOLVE_PROTOCOL_VERSION = 1;

/** Renderer-side completion-details resolver over a shared Worker port. */
export class LanguageCompletionResolveWireClient extends DisposableOwner implements LanguageCompletionItemResolver {
	private readonly pending = new Map<number, PendingResolveRequest>();
	private nextRequestId = 1;
	private failure: Error | undefined;

	constructor(
		private readonly port: LanguageWorkerWirePort,
		private readonly invalidateWorker: (error: Error) => void,
	) {
		super();
		assertPort(port);
		if (typeof invalidateWorker !== "function") {
			throw new TypeError("Completion resolve wire client requires an invalidation callback");
		}
		this.own(port.onMessage(message => this.receive(message)));
		this.defer(() => {
			this.failPending(new ReferenceError("LanguageCompletionResolveWireClient is already disposed"));
		});
	}

	resolveCompletionItem(request: LanguageCompletionResolveRequest, signal: AbortSignal): Promise<LanguageCompletionItemDetails> {
		this.ensureAlive();
		const target = normalizeLanguageCompletionResolveRequest(request);
		signal.throwIfAborted();
		const requestId = this.nextRequestId++;
		return new Promise((resolve, reject) => {
			const abort = (): void => {
				const pending = this.pending.get(requestId);
				if (!pending) return;
				this.pending.delete(requestId);
				pending.removeAbort();
				try {
					this.port.send(Object.freeze({
						protocol: RESOLVE_PROTOCOL,
						version: RESOLVE_PROTOCOL_VERSION,
						kind: "cancel",
						requestId,
					}));
				} catch {
					// The local cancellation outcome remains authoritative.
				}
				reject(abortError(signal.reason));
			};
			signal.addEventListener("abort", abort, { once: true });
			const pending: PendingResolveRequest = {
				target,
				resolve,
				reject,
				removeAbort: () => signal.removeEventListener("abort", abort),
			};
			this.pending.set(requestId, pending);
			try {
				this.port.send(Object.freeze({
					protocol: RESOLVE_PROTOCOL,
					version: RESOLVE_PROTOCOL_VERSION,
					kind: "resolve",
					requestId,
					target,
				}));
			} catch (error) {
				this.pending.delete(requestId);
				pending.removeAbort();
				reject(error);
			}
		});
	}

	invalidate(error: Error): void {
		if (this.failure) return;
		this.failure = error;
		this.failPending(error);
		try {
			this.invalidateWorker(error);
		} catch {
			// The original resolve protocol failure remains authoritative.
		}
	}

	private receive(value: unknown): void {
		if (!isResolveMessage(value)) return;
		try {
			assertEnvelope(value);
			const requestId = readRequestId(value.requestId);
			const pending = this.pending.get(requestId);
			if (!pending) return;
			if (value.kind === "failure") {
				const error = decodeRemoteError(value.error);
				this.pending.delete(requestId);
				pending.removeAbort();
				pending.reject(error);
				return;
			}
			if (value.kind !== "result") {
				throw new TypeError(`Unknown completion resolve response '${String(value.kind)}'`);
			}
			const target = normalizeLanguageCompletionResolveRequest(value.target as LanguageCompletionResolveRequest);
			if (!resolveTargetsEqual(target, pending.target)) {
				throw new Error("Completion resolve response does not match its request");
			}
			const details = normalizeLanguageCompletionItemDetails(value.details);
			this.pending.delete(requestId);
			pending.removeAbort();
			pending.resolve(details);
		} catch (error) {
			this.invalidate(asError(error));
		}
	}

	private failPending(error: Error): void {
		const pending = [...this.pending.values()];
		this.pending.clear();
		for (const request of pending) {
			request.removeAbort();
			request.reject(error);
		}
	}

	private ensureAlive(): void {
		this.assertNotDisposed();
		if (this.failure) throw this.failure;
	}
}

/** Worker-side dispatcher for deferred completion item details. */
export class LanguageCompletionResolveWireServer extends DisposableOwner {
	private readonly active = new Map<number, AbortController>();

	constructor(
		private readonly port: LanguageWorkerWirePort,
		private readonly resolver: LanguageCompletionItemResolver,
	) {
		super();
		assertPort(port);
		if (!resolver || typeof resolver.resolveCompletionItem !== "function") {
			throw new TypeError("Completion resolve wire server requires a resolver");
		}
		this.own(port.onMessage(message => this.receive(message)));
		this.defer(() => {
			for (const controller of this.active.values()) controller.abort("serverDisposed");
			this.active.clear();
		});
	}

	private receive(value: unknown): void {
		if (!isResolveMessage(value) || this.isDisposed) return;
		let requestId: number;
		try {
			assertEnvelope(value);
			requestId = readRequestId(value.requestId);
			if (value.kind === "cancel") {
				this.active.get(requestId)?.abort("clientCancelled");
				return;
			}
			if (value.kind !== "resolve") {
				throw new TypeError(`Unknown completion resolve request '${String(value.kind)}'`);
			}
			if (this.active.has(requestId)) {
				throw new RangeError(`Duplicate completion resolve request '${requestId}'`);
			}
			const target = normalizeLanguageCompletionResolveRequest(value.target as LanguageCompletionResolveRequest);
			void this.resolve(requestId, target);
		} catch (error) {
			const recoverableRequestId = tryReadRequestId(value.requestId);
			if (recoverableRequestId !== undefined) this.sendFailure(recoverableRequestId, error);
		}
	}

	private async resolve(requestId: number, target: LanguageCompletionResolveRequest): Promise<void> {
		const controller = new AbortController();
		this.active.set(requestId, controller);
		try {
			const details = await this.resolver.resolveCompletionItem(target, controller.signal);
			controller.signal.throwIfAborted();
			if (!this.isDisposed) {
				this.port.send(Object.freeze({
					protocol: RESOLVE_PROTOCOL,
					version: RESOLVE_PROTOCOL_VERSION,
					kind: "result",
					requestId,
					target,
					details: normalizeLanguageCompletionItemDetails(details),
				}));
			}
		} catch (error) {
			if (!this.isDisposed && !controller.signal.aborted) this.sendFailure(requestId, error);
		} finally {
			if (this.active.get(requestId) === controller) this.active.delete(requestId);
		}
	}

	private sendFailure(requestId: number, error: unknown): void {
		const normalized = asError(error);
		this.port.send(Object.freeze({
			protocol: RESOLVE_PROTOCOL,
			version: RESOLVE_PROTOCOL_VERSION,
			kind: "failure",
			requestId,
			error: Object.freeze({ name: normalized.name, message: normalized.message }),
		}));
	}
}

interface PendingResolveRequest {
	readonly target: LanguageCompletionResolveRequest;
	readonly resolve: (details: LanguageCompletionItemDetails) => void;
	readonly reject: (error: unknown) => void;
	readonly removeAbort: () => void;
}

function isResolveMessage(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && (value as Record<string, unknown>).protocol === RESOLVE_PROTOCOL;
}

function assertEnvelope(value: Record<string, unknown>): void {
	if (value.version !== RESOLVE_PROTOCOL_VERSION) {
		throw new RangeError(`Unsupported completion resolve protocol version '${String(value.version)}'`);
	}
}

function decodeRemoteError(value: unknown): Error {
	if (typeof value !== "object" || value === null) {
		throw new TypeError("Completion resolve failure must be an object");
	}
	const dto = value as { readonly name?: unknown; readonly message?: unknown };
	if (typeof dto.name !== "string" || typeof dto.message !== "string") {
		throw new TypeError("Completion resolve failure must contain name and message");
	}
	const error = new Error(dto.message);
	error.name = dto.name;
	return error;
}

function resolveTargetsEqual(left: LanguageCompletionResolveRequest, right: LanguageCompletionResolveRequest): boolean {
	return left.completionRequestId === right.completionRequestId &&
		left.modelVersion === right.modelVersion &&
		left.providerId === right.providerId &&
		left.itemId === right.itemId;
}

function readRequestId(value: unknown): number {
	const result = tryReadRequestId(value);
	if (result === undefined) throw new RangeError("Completion resolve wire request ID must be a positive safe integer");
	return result;
}

function tryReadRequestId(value: unknown): number | undefined {
	return Number.isSafeInteger(value) && (value as number) > 0 ? value as number : undefined;
}

function assertPort(port: LanguageWorkerWirePort): void {
	if (!port || typeof port.send !== "function" || typeof port.onMessage !== "function") {
		throw new TypeError("Completion resolve wire port is invalid");
	}
}

function abortError(reason: unknown): Error {
	if (reason instanceof Error) return reason;
	const error = new Error(reason === undefined ? "Completion resolve request was cancelled" : String(reason));
	error.name = "AbortError";
	return error;
}

function asError(value: unknown): Error {
	return value instanceof Error ? value : new Error(String(value));
}
