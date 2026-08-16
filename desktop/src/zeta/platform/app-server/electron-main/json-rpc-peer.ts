import type { ChildProcessWithoutNullStreams } from "node:child_process";
import { CancellationError } from "../../../base/common/cancellation.js";
import {
  DisposableStore,
  type IDisposable,
  markAsDisposed,
  setDisposableOwner,
  trackDisposable,
  toDisposable,
} from "../../../base/common/lifecycle.js";
import { AppServerRemoteError } from "../common/appServerError.js";
import { ChildProcessJsonlTransport } from "./child-process-jsonl-transport.js";

type JsonRpcId = number | string;

export interface RpcMethodDefinition<P, R> {
  readonly method: string;
  readonly __params?: P;
  readonly __result?: R;
}

export interface RpcNotificationDefinition<P> {
  readonly method: string;
  readonly __params?: P;
}

export interface RpcRequestOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
}

export interface RpcRequestContext {
  readonly signal: AbortSignal;
}

export interface JsonRpcPeerOptions {
  maxPendingRequests?: number;
  retiredRequestLimit?: number;
}

export class RpcRequestCancelledError extends CancellationError {
  constructor(
    readonly method: string,
    reason?: unknown,
  ) {
    super(`JSON-RPC request cancelled: ${method}`, reason);
    this.name = "RpcRequestCancelledError";
  }
}

interface PendingRequest {
  resolve(value: unknown): void;
  reject(error: Error): void;
  timeout?: NodeJS.Timeout;
  abortListener?: IDisposable;
}

interface RequestHandler {
  (params: unknown, context: RpcRequestContext): unknown | Promise<unknown>;
}

type RetiredRequest = "completed" | "abandoned";

/**
 * Implements transport-independent JSON-RPC pairing, lifecycle, and bidirectional dispatch.
 */
export class JsonRpcPeer implements IDisposable {
  private readonly transport: ChildProcessJsonlTransport;
  private readonly subscriptions = new DisposableStore();
  private readonly maxPendingRequests: number;
  private readonly retiredRequestLimit: number;
  private readonly pending = new Map<number, PendingRequest>();
  private readonly retired = new Map<number, RetiredRequest>();
  private readonly notificationListeners = new Map<string, Set<(params: unknown) => void>>();
  private readonly requestHandlers = new Map<string, RequestHandler>();
  private readonly inboundRequests = new Map<JsonRpcId, AbortController>();
  private nextId = 1;
  private closedError?: Error;

  constructor(
    process: ChildProcessWithoutNullStreams,
    options: JsonRpcPeerOptions = {},
  ) {
    this.maxPendingRequests = positiveInteger(
      options.maxPendingRequests,
      128,
      "maxPendingRequests",
    );
    this.retiredRequestLimit = positiveInteger(
      options.retiredRequestLimit,
      1_024,
      "retiredRequestLimit",
    );
    this.transport = new ChildProcessJsonlTransport(process);
    this.subscriptions.add(
      this.transport.onFrame((frame) => this.onFrame(frame)),
    );
    this.subscriptions.add(
      this.transport.onClose((error) => this.shutdown(error)),
    );
    trackDisposable(this);
    setDisposableOwner(this.transport, this);
    setDisposableOwner(this.subscriptions, this);
  }

  request<P, R>(
    definition: RpcMethodDefinition<P, R>,
    params: P,
    options: RpcRequestOptions = {},
  ): Promise<R> {
    if (this.closedError) return Promise.reject(this.closedError);
    if (options.signal?.aborted) {
      return Promise.reject(
        new RpcRequestCancelledError(definition.method, options.signal.reason),
      );
    }
    if (this.pending.size >= this.maxPendingRequests) {
      return Promise.reject(new Error("JSON-RPC pending request limit reached"));
    }
    if (
      options.timeoutMs !== undefined &&
      (!Number.isSafeInteger(options.timeoutMs) || options.timeoutMs <= 0)
    ) {
      return Promise.reject(new Error("timeoutMs must be a positive safe integer"));
    }

    const id = this.nextId++;
    const promise = new Promise<R>((resolve, reject) => {
      const pending: PendingRequest = {
        resolve: (value) => resolve(value as R),
        reject,
      };
      if (options.timeoutMs !== undefined) {
        pending.timeout = setTimeout(() => {
          this.abandon(id, new Error(`JSON-RPC request timed out: ${definition.method}`));
        }, options.timeoutMs);
        pending.timeout.unref();
      }
      if (options.signal) {
        const signal = options.signal;
        const onAbort = (): void => {
          this.cancelOutbound(id, definition.method, signal.reason);
        };
        signal.addEventListener("abort", onAbort, { once: true });
        pending.abortListener = toDisposable(() => {
          signal.removeEventListener("abort", onAbort);
        });
      }
      this.pending.set(id, pending);
    });

    const frame = JSON.stringify({
      jsonrpc: "2.0",
      id,
      method: definition.method,
      params,
    });
    this.transport.send(frame).catch((error: Error) => {
      this.abandon(id, error);
    });
    return promise;
  }

  notify<P>(definition: RpcNotificationDefinition<P>, params: P): Promise<void> {
    if (this.closedError) return Promise.reject(this.closedError);
    return this.transport.send(
      JSON.stringify({ jsonrpc: "2.0", method: definition.method, params }),
    );
  }

  onNotification<P>(
    definition: RpcNotificationDefinition<P>,
    listener: (params: P) => void,
  ): IDisposable {
    if (this.closedError) throw this.closedError;
    let listeners = this.notificationListeners.get(definition.method);
    if (!listeners) {
      listeners = new Set();
      this.notificationListeners.set(definition.method, listeners);
    }
    const untypedListener = listener as (params: unknown) => void;
    listeners.add(untypedListener);
    return toDisposable(() => {
      listeners?.delete(untypedListener);
      if (listeners?.size === 0) this.notificationListeners.delete(definition.method);
    });
  }

  registerRequestHandler<P, R>(
    definition: RpcMethodDefinition<P, R>,
    handler: (params: P, context: RpcRequestContext) => R | Promise<R>,
  ): IDisposable {
    if (this.closedError) throw this.closedError;
    if (this.requestHandlers.has(definition.method)) {
      throw new Error(`JSON-RPC request handler already registered: ${definition.method}`);
    }
    const untypedHandler: RequestHandler = (params, context) =>
      handler(params as P, context);
    this.requestHandlers.set(definition.method, untypedHandler);
    return toDisposable(() => {
      if (this.requestHandlers.get(definition.method) === untypedHandler) {
        this.requestHandlers.delete(definition.method);
      }
    });
  }

  diagnostics(): string {
    return this.transport.diagnostics();
  }

  close(): Promise<void> {
    this.shutdown(new Error("JSON-RPC peer closed"));
    return this.transport.close();
  }

  dispose(): void {
    this.shutdown(new Error("JSON-RPC peer disposed"));
    this.transport.dispose();
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  private onFrame(frame: string): void {
    let message: unknown;
    try {
      message = JSON.parse(frame);
    } catch {
      this.protocolFailure("App Server emitted invalid JSON");
      return;
    }
    if (!isObject(message) || message.jsonrpc !== "2.0") {
      this.protocolFailure("App Server emitted an invalid JSON-RPC envelope");
      return;
    }
    if (typeof message.method === "string") {
      this.onInboundCall(message);
      return;
    }
    this.onResponse(message);
  }

  private onResponse(message: Record<string, unknown>): void {
    if (!Number.isSafeInteger(message.id) || (message.id as number) <= 0) {
      this.protocolFailure("JSON-RPC response has an invalid request ID");
      return;
    }
    const id = message.id as number;
    const hasResult = Object.hasOwn(message, "result");
    const hasError = Object.hasOwn(message, "error");
    if (hasResult === hasError) {
      this.protocolFailure("JSON-RPC response must contain exactly one of result or error");
      return;
    }

    const pending = this.pending.get(id);
    if (!pending) {
      const retired = this.retired.get(id);
      if (retired === "abandoned") return;
      this.protocolFailure(
        retired === "completed"
          ? `App Server emitted a duplicate response for request ${id}`
          : `App Server emitted a response for unknown request ${id}`,
      );
      return;
    }
    this.pending.delete(id);
    cleanupPending(pending);
    this.retire(id, "completed");

    if (hasError) {
      const error = message.error;
      if (
        !isObject(error) ||
        !Number.isInteger(error.code) ||
        typeof error.message !== "string" ||
        error.data !== null
      ) {
        pending.reject(new Error("App Server emitted an invalid JSON-RPC error"));
        this.protocolFailure("App Server emitted an invalid JSON-RPC error");
        return;
      }
      pending.reject(new AppServerRemoteError(error.code as number, error.message, error.data));
      return;
    }
    pending.resolve(message.result);
  }

  private onInboundCall(message: Record<string, unknown>): void {
    if (!Object.hasOwn(message, "params")) {
      this.protocolFailure("JSON-RPC call is missing params");
      return;
    }
    const method = message.method as string;
    const hasId = Object.hasOwn(message, "id");
    if (!hasId) {
      if (method === "$/cancelRequest") {
        this.cancelInboundRequest(message.params);
        return;
      }
      const listeners = this.notificationListeners.get(method);
      if (!listeners) return;
      for (const listener of listeners) {
        try {
          listener(message.params);
        } catch {
          // Listener isolation: one presentation listener cannot break the connection or peers.
        }
      }
      return;
    }

    if (!isJsonRpcId(message.id)) {
      this.protocolFailure("Inbound JSON-RPC request has an invalid ID");
      return;
    }
    const id = message.id;
    if (
      this.inboundRequests.has(id) ||
      this.inboundRequests.size >= this.maxPendingRequests
    ) {
      void this.sendError(id, -32600, "Invalid Request", null).catch(() => {});
      return;
    }
    const handler = this.requestHandlers.get(method);
    if (!handler) {
      void this.sendError(id, -32601, "Method not found", null).catch(() => {});
      return;
    }
    const controller = new AbortController();
    this.inboundRequests.set(id, controller);
    void this.runInboundHandler(id, handler, message.params, controller);
  }

  private async runInboundHandler(
    id: JsonRpcId,
    handler: RequestHandler,
    params: unknown,
    controller: AbortController,
  ): Promise<void> {
    let response: unknown;
    try {
      const result = await handler(params, { signal: controller.signal });
      if (controller.signal.aborted) {
        response = {
          jsonrpc: "2.0",
          id,
          error: { code: -32800, message: "Request cancelled", data: null },
        };
      } else {
        response = { jsonrpc: "2.0", id, result };
      }
    } catch (error) {
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
      await this.transport.send(JSON.stringify(response));
    } catch {
      // Transport shutdown already owns connection-wide cleanup.
    }
    this.inboundRequests.delete(id);
  }

  private cancelInboundRequest(params: unknown): void {
    if (!isObject(params) || !isJsonRpcId(params.id)) return;
    this.inboundRequests.get(params.id)?.abort();
  }

  private sendError(id: JsonRpcId, code: number, message: string, data: unknown): Promise<void> {
    return this.transport.send(
      JSON.stringify({ jsonrpc: "2.0", id, error: { code, message, data } }),
    );
  }

  private cancelOutbound(id: number, method: string, reason?: unknown): void {
    if (!this.abandon(id, new RpcRequestCancelledError(method, reason))) return;
    void this.transport.send(JSON.stringify({
      jsonrpc: "2.0",
      method: "$/cancelRequest",
      params: { id },
    })).catch(() => {
      // The local request is already settled; transport shutdown owns failures.
    });
  }

  private abandon(id: number, error: Error): boolean {
    const pending = this.pending.get(id);
    if (!pending) return false;
    this.pending.delete(id);
    cleanupPending(pending);
    this.retire(id, "abandoned");
    pending.reject(error);
    return true;
  }

  private retire(id: number, outcome: RetiredRequest): void {
    this.retired.set(id, outcome);
    while (this.retired.size > this.retiredRequestLimit) {
      const oldest = this.retired.keys().next().value as number | undefined;
      if (oldest === undefined) break;
      this.retired.delete(oldest);
    }
  }

  private protocolFailure(message: string): void {
    const error = new Error(message);
    this.shutdown(error);
    void this.transport.close();
  }

  private shutdown(error: Error): void {
    if (this.closedError) return;
    this.closedError = error;
    this.subscriptions.dispose();
    for (const pending of this.pending.values()) {
      cleanupPending(pending);
      pending.reject(error);
    }
    this.pending.clear();
    for (const controller of this.inboundRequests.values()) {
      controller.abort(error);
    }
    this.inboundRequests.clear();
    this.notificationListeners.clear();
    this.requestHandlers.clear();
    markAsDisposed(this);
  }
}

function cleanupPending(pending: PendingRequest): void {
  if (pending.timeout) clearTimeout(pending.timeout);
  pending.abortListener?.dispose();
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isJsonRpcId(value: unknown): value is JsonRpcId {
  return typeof value === "string" || (Number.isSafeInteger(value) && (value as number) >= 0);
}

function positiveInteger(value: number | undefined, fallback: number, name: string): number {
  const resolved = value ?? fallback;
  if (!Number.isSafeInteger(resolved) || resolved <= 0) {
    throw new Error(`${name} must be a positive safe integer`);
  }
  return resolved;
}
