import { APP_SERVER_METHODS, APP_SERVER_NOTIFICATIONS, APP_SERVER_SCHEMA_HASH, type AppServerMethod, type AppServerMethodDefinition, type InitializeResult, type MethodParams, type MethodResult, type ServerNotification } from "../../../../../generated/app-server/types.js";
import type { AppServerConnectionState } from "../common/appServerApi.js";
import type { DisposableHandle } from "../../ipc/common/ipc.js";

export const WEB_APP_SERVER_PROTOCOL_VERSION = 1;
export const WEB_APP_SERVER_CONNECT_EVENT = "zeta:app-server:connect";
export const WEB_APP_SERVER_CONNECTED_EVENT = "zeta:app-server:connected";
export const WEB_APP_SERVER_DISCONNECT_EVENT = "zeta:app-server:disconnect";
export const WEB_APP_SERVER_FRAME_EVENT = "zeta:app-server:frame";
export const WEB_APP_SERVER_CLOSED_EVENT = "zeta:app-server:closed";

const DEFAULT_CONNECT_TIMEOUT = 10_000;
const DEFAULT_REQUEST_TIMEOUT = 30_000;
const MAX_FRAME_BYTES = 320 * 1024 * 1024;

export interface ViteDevHotContext {
  on(event: string, listener: (payload: unknown) => void): void;
  off(event: string, listener: (payload: unknown) => void): void;
  send(event: string, payload?: unknown): void;
}

export interface ViteDevAppServerMetadata {
  readonly workspaceId: string;
  readonly workspaceRoot: string;
}

export interface ViteDevAppServerConnectionOptions {
  readonly clientName?: string;
  readonly clientVersion?: string;
  readonly connectTimeoutMs?: number;
  readonly requestTimeoutMs?: number;
}

interface PendingRequest {
  readonly method: string;
  readonly resolve: (value: unknown) => void;
  readonly reject: (error: Error) => void;
  readonly timeout: ReturnType<typeof setTimeout>;
}

/** Error returned by a remote App Server JSON-RPC operation. */
export class WebAppServerRemoteError extends Error {
  constructor(readonly code: number, message: string, readonly data: unknown) {
    super(message);
    this.name = "WebAppServerRemoteError";
  }
}

/** Owns one Vite WebSocket-backed development App Server connection. */
export class ViteDevAppServerConnection {
  private readonly hot: ViteDevHotContext;
  private readonly options: Required<ViteDevAppServerConnectionOptions>;
  private readonly stateListeners = new Set<(state: AppServerConnectionState) => void>();
  private readonly notificationListeners = new Set<(notification: ServerNotification) => void>();
  private readonly pending = new Map<number, PendingRequest>();
  private nextRequestId = 1;
  private _state: AppServerConnectionState = "stopped";
  private _slashCommands: readonly InitializeResult["slashCommands"][number][] = [];
  private metadata: ViteDevAppServerMetadata | undefined;
  private connectResolve: ((metadata: ViteDevAppServerMetadata) => void) | undefined;
  private connectReject: ((error: Error) => void) | undefined;
  private connectTimeout: ReturnType<typeof setTimeout> | undefined;
  private disposed = false;

  constructor(hot: ViteDevHotContext, options: ViteDevAppServerConnectionOptions = {}) {
    this.hot = hot;
    this.options = {
      clientName: options.clientName ?? "zeta-web",
      clientVersion: options.clientVersion ?? "0.1.0",
      connectTimeoutMs: positiveInteger(options.connectTimeoutMs, DEFAULT_CONNECT_TIMEOUT, "connectTimeoutMs"),
      requestTimeoutMs: positiveInteger(options.requestTimeoutMs, DEFAULT_REQUEST_TIMEOUT, "requestTimeoutMs"),
    };
    this.hot.on(WEB_APP_SERVER_CONNECTED_EVENT, this.handleConnected);
    this.hot.on(WEB_APP_SERVER_FRAME_EVENT, this.handleFrame);
    this.hot.on(WEB_APP_SERVER_CLOSED_EVENT, this.handleClosed);
  }

  get state(): AppServerConnectionState {
    return this._state;
  }

  get slashCommands(): readonly InitializeResult["slashCommands"][number][] {
    return this._slashCommands;
  }

  async connect(): Promise<ViteDevAppServerMetadata> {
    if (this.disposed) throw new Error("Cannot connect a disposed Web App Server client");
    if (this._state !== "stopped") throw new Error(`Cannot connect Web App Server client from ${this._state}`);
    this.setState("starting");
    const connected = new Promise<ViteDevAppServerMetadata>((resolve, reject) => {
      this.connectResolve = resolve;
      this.connectReject = reject;
      this.connectTimeout = setTimeout(() => this.fail(new Error("Timed out connecting to the Web App Server bridge")), this.options.connectTimeoutMs);
    });
    this.hot.send(WEB_APP_SERVER_CONNECT_EVENT, { protocolVersion: WEB_APP_SERVER_PROTOCOL_VERSION });
    const metadata = await connected;
    this.setState("initializing");
    try {
      const initialized = await this.requestRaw(APP_SERVER_METHODS.initialize, {
        clientInfo: { name: this.options.clientName, version: this.options.clientVersion },
        capabilities: { notifications: true },
      }, this.options.connectTimeoutMs);
      this._slashCommands = validateInitializeResult(initialized);
      this.setState("ready");
      return metadata;
    } catch (error) {
      this.fail(asError(error));
      throw error;
    }
  }

  request<M extends AppServerMethod>(definition: AppServerMethodDefinition<M>, params: MethodParams<M>): Promise<MethodResult<M>> {
    if (this._state !== "ready") return Promise.reject(new Error(`Web App Server is not ready: ${this._state}`));
    return this.requestRaw(definition, params, this.options.requestTimeoutMs);
  }

  onStateChange(listener: (state: AppServerConnectionState) => void): DisposableHandle {
    this.stateListeners.add(listener);
    return disposable(() => this.stateListeners.delete(listener));
  }

  onNotification(listener: (notification: ServerNotification) => void): DisposableHandle {
    this.notificationListeners.add(listener);
    return disposable(() => this.notificationListeners.delete(listener));
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    try {
      this.hot.send(WEB_APP_SERVER_DISCONNECT_EVENT, { protocolVersion: WEB_APP_SERVER_PROTOCOL_VERSION });
    } finally {
      this.hot.off(WEB_APP_SERVER_CONNECTED_EVENT, this.handleConnected);
      this.hot.off(WEB_APP_SERVER_FRAME_EVENT, this.handleFrame);
      this.hot.off(WEB_APP_SERVER_CLOSED_EVENT, this.handleClosed);
      this.shutdown(new Error("Web App Server client disposed"), "stopped");
      this.stateListeners.clear();
      this.notificationListeners.clear();
    }
  }

  private requestRaw<M extends AppServerMethod>(definition: AppServerMethodDefinition<M>, params: MethodParams<M>, timeoutMs: number): Promise<MethodResult<M>> {
    const id = this.nextRequestId++;
    const promise = new Promise<MethodResult<M>>((resolve, reject) => {
      const timeout = setTimeout(() => {
        const pending = this.pending.get(id);
        if (!pending) return;
        this.pending.delete(id);
        pending.reject(new Error(`Web App Server request timed out: ${definition.method}`));
      }, timeoutMs);
      this.pending.set(id, {
        method: definition.method,
        resolve: (value) => resolve(value as MethodResult<M>),
        reject,
        timeout,
      });
    });
    const frame = JSON.stringify({ jsonrpc: "2.0", id, method: definition.method, params });
    try {
      this.hot.send(WEB_APP_SERVER_FRAME_EVENT, { frame });
    } catch (error) {
      this.rejectPending(id, asError(error));
    }
    return promise;
  }

  private readonly handleConnected = (payload: unknown): void => {
    if (this._state !== "starting") return;
    try {
      const metadata = validateConnectedPayload(payload);
      this.metadata = metadata;
      this.clearConnectTimeout();
      const resolve = this.connectResolve;
      this.connectResolve = undefined;
      this.connectReject = undefined;
      resolve?.(metadata);
    } catch (error) {
      this.fail(asError(error));
    }
  };

  private readonly handleFrame = (payload: unknown): void => {
    try {
      const frame = validateFramePayload(payload);
      const message: unknown = JSON.parse(frame);
      if (!isRecord(message) || message.jsonrpc !== "2.0") throw new Error("App Server emitted an invalid JSON-RPC envelope");
      if (typeof message.method === "string") {
        this.handleInboundCall(message);
      } else {
        this.handleResponse(message);
      }
    } catch (error) {
      this.fail(asError(error));
    }
  };

  private readonly handleClosed = (payload: unknown): void => {
    const message = isRecord(payload) && typeof payload.message === "string" && payload.message.trim()
      ? payload.message
      : "Web App Server bridge closed";
    this.fail(new Error(message));
  };

  private handleResponse(message: Record<string, unknown>): void {
    if (!Number.isSafeInteger(message.id) || (message.id as number) <= 0) throw new Error("App Server emitted an invalid response ID");
    const id = message.id as number;
    const pending = this.pending.get(id);
    if (!pending) return;
    this.pending.delete(id);
    clearTimeout(pending.timeout);
    const hasResult = Object.hasOwn(message, "result");
    const hasError = Object.hasOwn(message, "error");
    if (hasResult === hasError) throw new Error("App Server response must contain exactly one of result or error");
    if (hasError) {
      const error = message.error;
      if (!isRecord(error) || !Number.isInteger(error.code) || typeof error.message !== "string") {
        pending.reject(new Error("App Server emitted an invalid JSON-RPC error"));
        return;
      }
      pending.reject(new WebAppServerRemoteError(error.code as number, error.message, error.data));
      return;
    }
    pending.resolve(message.result);
  }

  private handleInboundCall(message: Record<string, unknown>): void {
    if (Object.hasOwn(message, "id")) {
      const frame = JSON.stringify({ jsonrpc: "2.0", id: message.id, error: { code: -32601, message: "Method not found", data: null } });
      this.hot.send(WEB_APP_SERVER_FRAME_EVENT, { frame });
      return;
    }
    if (!Object.hasOwn(message, "params") || !serverNotificationMethods.has(message.method as string)) return;
    const notification = { method: message.method, params: message.params } as ServerNotification;
    for (const listener of this.notificationListeners) {
      try {
        listener(notification);
      } catch {
        // One presentation listener cannot break connection-wide delivery.
      }
    }
  }

  private rejectPending(id: number, error: Error): void {
    const pending = this.pending.get(id);
    if (!pending) return;
    this.pending.delete(id);
    clearTimeout(pending.timeout);
    pending.reject(error);
  }

  private fail(error: Error): void {
    this.shutdown(error, this._state === "stopped" ? "stopped" : "crashed");
  }

  private shutdown(error: Error, state: AppServerConnectionState): void {
    this.clearConnectTimeout();
    const reject = this.connectReject;
    this.connectResolve = undefined;
    this.connectReject = undefined;
    reject?.(error);
    for (const [id] of this.pending) this.rejectPending(id, error);
    this.setState(state);
  }

  private clearConnectTimeout(): void {
    if (this.connectTimeout !== undefined) clearTimeout(this.connectTimeout);
    this.connectTimeout = undefined;
  }

  private setState(state: AppServerConnectionState): void {
    if (this._state === state) return;
    this._state = state;
    for (const listener of this.stateListeners) {
      try {
        listener(state);
      } catch {
        // State observers are isolated from the connection lifecycle.
      }
    }
  }
}

const serverNotificationMethods: ReadonlySet<string> = new Set(Object.values(APP_SERVER_NOTIFICATIONS).map((definition) => definition.method));

function validateConnectedPayload(payload: unknown): ViteDevAppServerMetadata {
  if (!isRecord(payload) || payload.protocolVersion !== WEB_APP_SERVER_PROTOCOL_VERSION) {
    throw new Error("Web App Server bridge protocol version mismatch");
  }
  if (typeof payload.workspaceId !== "string" || payload.workspaceId.trim().length === 0) throw new Error("Web App Server bridge workspace ID is invalid");
  if (typeof payload.workspaceRoot !== "string" || payload.workspaceRoot.trim().length === 0) throw new Error("Web App Server bridge workspace root is invalid");
  return { workspaceId: payload.workspaceId, workspaceRoot: payload.workspaceRoot };
}

function validateFramePayload(payload: unknown): string {
  if (!isRecord(payload) || typeof payload.frame !== "string") throw new Error("Web App Server bridge frame is invalid");
  if (new TextEncoder().encode(payload.frame).byteLength > MAX_FRAME_BYTES) throw new Error(`Web App Server frame exceeds ${MAX_FRAME_BYTES} bytes`);
  return payload.frame;
}

function validateInitializeResult(value: InitializeResult): readonly InitializeResult["slashCommands"][number][] {
  const result = value as unknown;
  const serverInfo = isRecord(result) && isRecord(result.serverInfo) ? result.serverInfo : undefined;
  const serverName = serverInfo?.name;
  const schemaHash = isRecord(result) ? result.schemaHash : undefined;
  if (serverName !== "zeta-app-server" || schemaHash !== APP_SERVER_SCHEMA_HASH) {
    throw new Error(`Web App Server initialization identity or schema mismatch (received server ${describeValue(serverName)}, schema ${describeValue(schemaHash)})`);
  }
  const capabilities = isRecord(result) ? result.capabilities : undefined;
  if (!isRecord(capabilities) || typeof capabilities.sessions !== "boolean" || typeof capabilities.threads !== "boolean" || typeof capabilities.turns !== "boolean") {
    throw new Error("Web App Server initialize result is malformed");
  }
  const slashCommands = isRecord(result) ? result.slashCommands : undefined;
  if (!Array.isArray(slashCommands) || slashCommands.some((command) => !isRecord(command) || typeof command.name !== "string" || typeof command.description !== "string" || (command.argumentMode !== "none" && command.argumentMode !== "optional"))) {
    throw new Error("Web App Server initialize slash commands are malformed");
  }
  return slashCommands as readonly InitializeResult["slashCommands"][number][];
}

function disposable(dispose: () => void): DisposableHandle {
  let active = true;
  return {
    dispose: () => {
      if (!active) return;
      active = false;
      dispose();
    },
  };
}

function positiveInteger(value: number | undefined, fallback: number, name: string): number {
  const resolved = value ?? fallback;
  if (!Number.isSafeInteger(resolved) || resolved <= 0) throw new RangeError(`${name} must be a positive safe integer`);
  return resolved;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function describeValue(value: unknown): string {
  return typeof value === "string" ? JSON.stringify(value) : String(value);
}

function asError(value: unknown): Error {
  return value instanceof Error ? value : new Error(String(value));
}
