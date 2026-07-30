import type {
  AppServerMethod,
  AppServerMethodDefinition,
  AppServerNotificationDefinition,
  AppServerNotificationMethod,
  InitializeResult,
  MethodParams,
  MethodResult,
  NotificationParams,
  ServerCapabilities,
  ServerNotification,
} from "../../../../../generated/app-server/types.js";
import {
  APP_SERVER_METHODS,
} from "../../../../../generated/app-server/types.js";
import {
  type IDisposable,
  markAsDisposed,
  setDisposableOwner,
  trackDisposable,
} from "../../../base/common/lifecycle.js";
import { assertDefined } from "../../../base/common/types.js";
import { AppServerClient } from "./app-server-client.js";
import type {
  RpcMethodDefinition,
  RpcRequestContext,
  RpcRequestOptions,
} from "./json-rpc-peer.js";

export type AppServerSessionState = "created" | "initializing" | "ready" | "closed";

export interface AppServerSessionOptions {
  clientName: string;
  clientVersion: string;
  schemaHash: string;
  initializeTimeoutMs: number;
  expectedServerName?: string;
}

/**
 * Owns one initialized App Server connection and its negotiated immutable capabilities.
 */
export class AppServerSession implements IDisposable {
  private _state: AppServerSessionState = "created";
  private _initializeResult: InitializeResult | undefined;

  constructor(
    readonly client: AppServerClient,
    readonly options: AppServerSessionOptions,
  ) {
    trackDisposable(this);
    setDisposableOwner(client, this);
  }

  get state(): AppServerSessionState {
    return this._state;
  }

  get capabilities(): ServerCapabilities {
    return this.initialization.capabilities;
  }

  get serverInfo(): InitializeResult["serverInfo"] {
    return this.initialization.serverInfo;
  }

  async initialize(): Promise<InitializeResult> {
    if (this._state !== "created") {
      throw new Error(`Cannot initialize App Server session from ${this._state}`);
    }
    this._state = "initializing";
    try {
      const initialized = await this.client.request(
        APP_SERVER_METHODS.initialize,
        {
          clientInfo: {
            name: this.options.clientName,
            version: this.options.clientVersion,
          },
          capabilities: { notifications: true },
        },
        { timeoutMs: this.options.initializeTimeoutMs },
      );
      validateInitializeResult(initialized);
      if (
        this.options.expectedServerName &&
        initialized.serverInfo.name !== this.options.expectedServerName
      ) {
        throw new Error(
          `Unexpected App Server identity: ${initialized.serverInfo.name}`,
        );
      }
      if (initialized.schemaHash !== this.options.schemaHash) {
        throw new Error(
          `Zeta app-server schema mismatch: expected ${this.options.schemaHash}, received ${initialized.schemaHash}`,
        );
      }
      this._initializeResult = initialized;
      this._state = "ready";
      return initialized;
    } catch (error) {
      await this.close();
      throw error;
    }
  }

  request<M extends AppServerMethod>(
    definition: AppServerMethodDefinition<M>,
    params: MethodParams<M>,
    options?: RpcRequestOptions,
  ): Promise<MethodResult<M>> {
    if (this._state !== "ready") {
      return Promise.reject(new Error(`App Server session is not ready: ${this._state}`));
    }
    return this.client.request(definition, params, options);
  }

  onNotification<M extends AppServerNotificationMethod>(
    definition: AppServerNotificationDefinition<M>,
    listener: (params: NotificationParams<M>) => void,
  ): IDisposable {
    return this.client.onNotification(definition, listener);
  }

  onAnyNotification(
    listener: (notification: ServerNotification) => void,
  ): IDisposable {
    return this.client.onAnyNotification(listener);
  }

  registerRequestHandler<P, R>(
    definition: RpcMethodDefinition<P, R>,
    handler: (params: P, context: RpcRequestContext) => R | Promise<R>,
  ): IDisposable {
    if (this._state === "closed") {
      throw new Error("Cannot register a handler on a closed App Server session");
    }
    return this.client.peer.registerRequestHandler(definition, handler);
  }

  diagnostics(): string {
    return this.client.diagnostics();
  }

  async close(): Promise<void> {
    if (this._state === "closed") return;
    this._state = "closed";
    try {
      await this.client.close();
    } finally {
      markAsDisposed(this);
    }
  }

  dispose(): void {
    if (this._state === "closed") return;
    this._state = "closed";
    try {
      this.client.dispose();
    } finally {
      markAsDisposed(this);
    }
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  private get initialization(): InitializeResult {
    assertDefined(this._initializeResult, "App Server session is not initialized");
    return this._initializeResult;
  }
}

function validateInitializeResult(value: InitializeResult): void {
  if (
    !value ||
    typeof value !== "object" ||
    !value.serverInfo ||
    typeof value.serverInfo.name !== "string" ||
    value.serverInfo.name.trim().length === 0 ||
    typeof value.serverInfo.version !== "string" ||
    value.serverInfo.version.trim().length === 0 ||
    typeof value.schemaHash !== "string" ||
    !value.capabilities ||
    typeof value.capabilities.sessions !== "boolean" ||
    typeof value.capabilities.threads !== "boolean" ||
    typeof value.capabilities.turns !== "boolean" ||
    typeof value.capabilities.resources !== "boolean" ||
    typeof value.capabilities.fileSystem !== "boolean" ||
    typeof value.capabilities.workspaceSearch !== "boolean" ||
    typeof value.capabilities.terminal !== "boolean" ||
    typeof value.capabilities.typst !== "boolean" ||
    typeof value.capabilities.updateReplay !== "boolean"
  ) {
    throw new Error("App Server initialize result is malformed");
  }
}
