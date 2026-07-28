import { CancellationError } from "../../../base/common/cancellation.js";
import { DisposableStore, markAsDisposed, setDisposableOwner, trackDisposable, toDisposable, } from "../../../base/common/lifecycle.js";
import { ChildProcessJsonlTransport } from "./child-process-jsonl-transport.js";
export class JsonRpcRemoteError extends Error {
    code;
    data;
    constructor(code, message, data) {
        super(message);
        this.code = code;
        this.data = data;
        this.name = "JsonRpcRemoteError";
    }
}
export class RpcRequestCancelledError extends CancellationError {
    method;
    constructor(method, reason) {
        super(`JSON-RPC request cancelled: ${method}`, reason);
        this.method = method;
        this.name = "RpcRequestCancelledError";
    }
}
/**
 * Implements transport-independent JSON-RPC pairing, lifecycle, and bidirectional dispatch.
 */
export class JsonRpcPeer {
    #transport;
    #subscriptions = new DisposableStore();
    #maxPendingRequests;
    #retiredRequestLimit;
    #pending = new Map();
    #retired = new Map();
    #notificationListeners = new Map();
    #requestHandlers = new Map();
    #inboundRequests = new Map();
    #nextId = 1;
    #closedError;
    constructor(process, options = {}) {
        this.#maxPendingRequests = positiveInteger(options.maxPendingRequests, 128, "maxPendingRequests");
        this.#retiredRequestLimit = positiveInteger(options.retiredRequestLimit, 1_024, "retiredRequestLimit");
        this.#transport = new ChildProcessJsonlTransport(process);
        this.#subscriptions.add(this.#transport.onFrame((frame) => this.#onFrame(frame)));
        this.#subscriptions.add(this.#transport.onClose((error) => this.#shutdown(error)));
        trackDisposable(this);
        setDisposableOwner(this.#transport, this);
        setDisposableOwner(this.#subscriptions, this);
    }
    request(definition, params, options = {}) {
        if (this.#closedError)
            return Promise.reject(this.#closedError);
        if (options.signal?.aborted) {
            return Promise.reject(new RpcRequestCancelledError(definition.method, options.signal.reason));
        }
        if (this.#pending.size >= this.#maxPendingRequests) {
            return Promise.reject(new Error("JSON-RPC pending request limit reached"));
        }
        if (options.timeoutMs !== undefined &&
            (!Number.isSafeInteger(options.timeoutMs) || options.timeoutMs <= 0)) {
            return Promise.reject(new Error("timeoutMs must be a positive safe integer"));
        }
        const id = this.#nextId++;
        const promise = new Promise((resolve, reject) => {
            const pending = {
                resolve: (value) => resolve(value),
                reject,
            };
            if (options.timeoutMs !== undefined) {
                pending.timeout = setTimeout(() => {
                    this.#abandon(id, new Error(`JSON-RPC request timed out: ${definition.method}`));
                }, options.timeoutMs);
                pending.timeout.unref();
            }
            if (options.signal) {
                const signal = options.signal;
                const onAbort = () => {
                    this.#cancelOutbound(id, definition.method, signal.reason);
                };
                signal.addEventListener("abort", onAbort, { once: true });
                pending.abortListener = toDisposable(() => {
                    signal.removeEventListener("abort", onAbort);
                });
            }
            this.#pending.set(id, pending);
        });
        const frame = JSON.stringify({
            jsonrpc: "2.0",
            id,
            method: definition.method,
            params,
        });
        this.#transport.send(frame).catch((error) => {
            this.#abandon(id, error);
        });
        return promise;
    }
    notify(definition, params) {
        if (this.#closedError)
            return Promise.reject(this.#closedError);
        return this.#transport.send(JSON.stringify({ jsonrpc: "2.0", method: definition.method, params }));
    }
    onNotification(definition, listener) {
        if (this.#closedError)
            throw this.#closedError;
        let listeners = this.#notificationListeners.get(definition.method);
        if (!listeners) {
            listeners = new Set();
            this.#notificationListeners.set(definition.method, listeners);
        }
        const untypedListener = listener;
        listeners.add(untypedListener);
        return toDisposable(() => {
            listeners?.delete(untypedListener);
            if (listeners?.size === 0)
                this.#notificationListeners.delete(definition.method);
        });
    }
    registerRequestHandler(definition, handler) {
        if (this.#closedError)
            throw this.#closedError;
        if (this.#requestHandlers.has(definition.method)) {
            throw new Error(`JSON-RPC request handler already registered: ${definition.method}`);
        }
        const untypedHandler = (params, context) => handler(params, context);
        this.#requestHandlers.set(definition.method, untypedHandler);
        return toDisposable(() => {
            if (this.#requestHandlers.get(definition.method) === untypedHandler) {
                this.#requestHandlers.delete(definition.method);
            }
        });
    }
    diagnostics() {
        return this.#transport.diagnostics();
    }
    close() {
        this.#shutdown(new Error("JSON-RPC peer closed"));
        return this.#transport.close();
    }
    dispose() {
        this.#shutdown(new Error("JSON-RPC peer disposed"));
        this.#transport.dispose();
    }
    [Symbol.dispose]() {
        this.dispose();
    }
    #onFrame(frame) {
        let message;
        try {
            message = JSON.parse(frame);
        }
        catch {
            this.#protocolFailure("App Server emitted invalid JSON");
            return;
        }
        if (!isObject(message) || message.jsonrpc !== "2.0") {
            this.#protocolFailure("App Server emitted an invalid JSON-RPC envelope");
            return;
        }
        if (typeof message.method === "string") {
            this.#onInboundCall(message);
            return;
        }
        this.#onResponse(message);
    }
    #onResponse(message) {
        if (!Number.isSafeInteger(message.id) || message.id <= 0) {
            this.#protocolFailure("JSON-RPC response has an invalid request ID");
            return;
        }
        const id = message.id;
        const hasResult = Object.hasOwn(message, "result");
        const hasError = Object.hasOwn(message, "error");
        if (hasResult === hasError) {
            this.#protocolFailure("JSON-RPC response must contain exactly one of result or error");
            return;
        }
        const pending = this.#pending.get(id);
        if (!pending) {
            const retired = this.#retired.get(id);
            if (retired === "abandoned")
                return;
            this.#protocolFailure(retired === "completed"
                ? `App Server emitted a duplicate response for request ${id}`
                : `App Server emitted a response for unknown request ${id}`);
            return;
        }
        this.#pending.delete(id);
        cleanupPending(pending);
        this.#retire(id, "completed");
        if (hasError) {
            const error = message.error;
            if (!isObject(error) ||
                !Number.isInteger(error.code) ||
                typeof error.message !== "string") {
                pending.reject(new Error("App Server emitted an invalid JSON-RPC error"));
                this.#protocolFailure("App Server emitted an invalid JSON-RPC error");
                return;
            }
            pending.reject(new JsonRpcRemoteError(error.code, error.message, error.data));
            return;
        }
        pending.resolve(message.result);
    }
    #onInboundCall(message) {
        if (!Object.hasOwn(message, "params")) {
            this.#protocolFailure("JSON-RPC call is missing params");
            return;
        }
        const method = message.method;
        const hasId = Object.hasOwn(message, "id");
        if (!hasId) {
            if (method === "$/cancelRequest") {
                this.#cancelInboundRequest(message.params);
                return;
            }
            const listeners = this.#notificationListeners.get(method);
            if (!listeners)
                return;
            for (const listener of listeners) {
                try {
                    listener(message.params);
                }
                catch {
                    // Listener isolation: one presentation listener cannot break the connection or peers.
                }
            }
            return;
        }
        if (!isJsonRpcId(message.id)) {
            this.#protocolFailure("Inbound JSON-RPC request has an invalid ID");
            return;
        }
        const id = message.id;
        if (this.#inboundRequests.has(id) ||
            this.#inboundRequests.size >= this.#maxPendingRequests) {
            void this.#sendError(id, -32600, "Invalid Request", null).catch(() => { });
            return;
        }
        const handler = this.#requestHandlers.get(method);
        if (!handler) {
            void this.#sendError(id, -32601, "Method not found", null).catch(() => { });
            return;
        }
        const controller = new AbortController();
        this.#inboundRequests.set(id, controller);
        void this.#runInboundHandler(id, handler, message.params, controller);
    }
    async #runInboundHandler(id, handler, params, controller) {
        let response;
        try {
            const result = await handler(params, { signal: controller.signal });
            if (controller.signal.aborted) {
                response = {
                    jsonrpc: "2.0",
                    id,
                    error: { code: -32800, message: "Request cancelled", data: null },
                };
            }
            else {
                response = { jsonrpc: "2.0", id, result };
            }
        }
        catch (error) {
            response = controller.signal.aborted
                ? {
                    jsonrpc: "2.0",
                    id,
                    error: { code: -32800, message: "Request cancelled", data: null },
                }
                : {
                    jsonrpc: "2.0",
                    id,
                    error: {
                        code: -32603,
                        message: error instanceof Error ? error.message : "Internal error",
                        data: null,
                    },
                };
        }
        try {
            await this.#transport.send(JSON.stringify(response));
        }
        catch {
            // Transport shutdown already owns connection-wide cleanup.
        }
        this.#inboundRequests.delete(id);
    }
    #cancelInboundRequest(params) {
        if (!isObject(params) || !isJsonRpcId(params.id))
            return;
        this.#inboundRequests.get(params.id)?.abort();
    }
    #sendError(id, code, message, data) {
        return this.#transport.send(JSON.stringify({ jsonrpc: "2.0", id, error: { code, message, data } }));
    }
    #cancelOutbound(id, method, reason) {
        if (!this.#abandon(id, new RpcRequestCancelledError(method, reason)))
            return;
        void this.#transport.send(JSON.stringify({
            jsonrpc: "2.0",
            method: "$/cancelRequest",
            params: { id },
        })).catch(() => {
            // The local request is already settled; transport shutdown owns failures.
        });
    }
    #abandon(id, error) {
        const pending = this.#pending.get(id);
        if (!pending)
            return false;
        this.#pending.delete(id);
        cleanupPending(pending);
        this.#retire(id, "abandoned");
        pending.reject(error);
        return true;
    }
    #retire(id, outcome) {
        this.#retired.set(id, outcome);
        while (this.#retired.size > this.#retiredRequestLimit) {
            const oldest = this.#retired.keys().next().value;
            if (oldest === undefined)
                break;
            this.#retired.delete(oldest);
        }
    }
    #protocolFailure(message) {
        const error = new Error(message);
        this.#shutdown(error);
        void this.#transport.close();
    }
    #shutdown(error) {
        if (this.#closedError)
            return;
        this.#closedError = error;
        this.#subscriptions.dispose();
        for (const pending of this.#pending.values()) {
            cleanupPending(pending);
            pending.reject(error);
        }
        this.#pending.clear();
        for (const controller of this.#inboundRequests.values()) {
            controller.abort(error);
        }
        this.#inboundRequests.clear();
        this.#notificationListeners.clear();
        this.#requestHandlers.clear();
        markAsDisposed(this);
    }
}
function cleanupPending(pending) {
    if (pending.timeout)
        clearTimeout(pending.timeout);
    pending.abortListener?.dispose();
}
function isObject(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}
function isJsonRpcId(value) {
    return typeof value === "string" || (Number.isSafeInteger(value) && value >= 0);
}
function positiveInteger(value, fallback, name) {
    const resolved = value ?? fallback;
    if (!Number.isSafeInteger(resolved) || resolved <= 0) {
        throw new Error(`${name} must be a positive safe integer`);
    }
    return resolved;
}
