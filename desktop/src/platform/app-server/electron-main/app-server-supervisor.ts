import {
  type ChildProcessWithoutNullStreams,
  spawn,
} from "node:child_process";
import { existsSync } from "node:fs";
import { isAbsolute } from "node:path";
import type {
  AppServerMethod,
  AppServerMethodDefinition,
  MethodParams,
  MethodResult,
  ServerNotification,
} from "../../../../generated/app-server/types.js";
import type { AppServerConnectionState } from "../common/renderer-api.js";
import { AppServerClient } from "./app-server-client.js";
import {
  AppServerSession,
  type AppServerSessionOptions,
} from "./app-server-session.js";
import { JsonRpcPeer, type RpcRequestOptions } from "./json-rpc-peer.js";

export interface SpawnAppServerOptions {
  environment: Readonly<Record<string, string>>;
}

export type SpawnAppServer = (
  executable: string,
  args: readonly string[],
  options: SpawnAppServerOptions,
) => ChildProcessWithoutNullStreams;

export interface AppServerSupervisorOptions {
  executable: string;
  args: readonly string[];
  environment: Readonly<Record<string, string>>;
  session: AppServerSessionOptions;
  maxRestartAttempts?: number;
  initialRestartDelayMs?: number;
  maxRestartDelayMs?: number;
  allowedEnvironmentKeys?: readonly string[];
  spawnProcess?: SpawnAppServer;
  fileExists?: (path: string) => boolean;
  wait?: (milliseconds: number) => Promise<void>;
}

type StateListener = (state: AppServerConnectionState) => void;
type NotificationListener = (notification: ServerNotification) => void;

/**
 * Supervises App Server process/session replacement without replaying application requests.
 */
export class AppServerSupervisor {
  readonly #spawnProcess: SpawnAppServer;
  readonly #fileExists: (path: string) => boolean;
  readonly #wait: (milliseconds: number) => Promise<void>;
  readonly #stateListeners = new Set<StateListener>();
  readonly #notificationListeners = new Set<NotificationListener>();
  readonly #maxRestartAttempts: number;
  readonly #initialRestartDelayMs: number;
  readonly #maxRestartDelayMs: number;
  #state: AppServerConnectionState = "stopped";
  #process?: ChildProcessWithoutNullStreams;
  #session?: AppServerSession;
  #generation = 0;
  #restartAttempts = 0;
  #stopping = false;
  #lastDiagnostics = "";

  constructor(readonly options: AppServerSupervisorOptions) {
    if (!isAbsolute(options.executable)) {
      throw new Error("App Server executable path must be absolute");
    }
    const allowedEnvironmentKeys = new Set(
      options.allowedEnvironmentKeys ?? ["PATH", "ZETA_STATE_ROOT"],
    );
    for (const key of Object.keys(options.environment)) {
      if (!allowedEnvironmentKeys.has(key)) {
        throw new Error(`App Server environment variable is not allowed: ${key}`);
      }
    }
    this.#maxRestartAttempts = nonNegativeInteger(
      options.maxRestartAttempts,
      3,
      "maxRestartAttempts",
    );
    this.#initialRestartDelayMs = positiveInteger(
      options.initialRestartDelayMs,
      250,
      "initialRestartDelayMs",
    );
    this.#maxRestartDelayMs = positiveInteger(
      options.maxRestartDelayMs,
      2_000,
      "maxRestartDelayMs",
    );
    this.#spawnProcess = options.spawnProcess ?? defaultSpawn;
    this.#fileExists = options.fileExists ?? existsSync;
    this.#wait = options.wait ?? wait;
  }

  get state(): AppServerConnectionState {
    return this.#state;
  }

  onStateChange(listener: StateListener): () => void {
    this.#stateListeners.add(listener);
    return () => this.#stateListeners.delete(listener);
  }

  onNotification(listener: NotificationListener): () => void {
    this.#notificationListeners.add(listener);
    return () => this.#notificationListeners.delete(listener);
  }

  async start(): Promise<void> {
    if (this.#state !== "stopped") {
      throw new Error(`Cannot start App Server supervisor from ${this.#state}`);
    }
    if (!this.#fileExists(this.options.executable)) {
      throw new Error(`Packaged Zeta binary is missing: ${this.options.executable}`);
    }
    this.#stopping = false;
    this.#restartAttempts = 0;
    let lastError: unknown;
    for (let attempt = 0; attempt <= this.#maxRestartAttempts; attempt += 1) {
      if (attempt > 0) {
        this.#setState("restarting");
        await this.#wait(this.#restartDelay(attempt - 1));
      }
      try {
        await this.#launch();
        return;
      } catch (error) {
        lastError = error;
        this.#setState("crashed");
      }
    }
    throw lastError instanceof Error
      ? lastError
      : new Error("App Server failed to start");
  }

  request<M extends AppServerMethod>(
    definition: AppServerMethodDefinition<M>,
    params: MethodParams<M>,
    options?: RpcRequestOptions,
  ): Promise<MethodResult<M>> {
    if (this.#state !== "ready" || !this.#session) {
      return Promise.reject(new Error(`App Server is not ready: ${this.#state}`));
    }
    return this.#session.request(definition, params, options);
  }

  diagnostics(): string {
    return this.#session?.diagnostics() ?? this.#lastDiagnostics;
  }

  async stop(): Promise<void> {
    if (this.#state === "stopped") return;
    this.#stopping = true;
    this.#generation += 1;
    this.#setState("stopping");
    const session = this.#session;
    this.#session = undefined;
    this.#process = undefined;
    await session?.close();
    this.#setState("stopped");
  }

  async #launch(): Promise<void> {
    this.#setState("starting");
    const generation = ++this.#generation;
    const child = this.#spawnProcess(
      this.options.executable,
      this.options.args,
      { environment: this.options.environment },
    );
    this.#process = child;
    child.once("exit", () => {
      if (this.#process !== child || this.#generation !== generation) return;
      const restart = !this.#stopping && this.#state === "ready";
      this.#lastDiagnostics = this.#session?.diagnostics() ?? this.#lastDiagnostics;
      this.#process = undefined;
      this.#session = undefined;
      if (restart) {
        this.#setState("crashed");
        void this.#restartAfterCrash();
      }
    });

    const session = new AppServerSession(
      new AppServerClient(new JsonRpcPeer(child)),
      this.options.session,
    );
    this.#session = session;
    session.onAnyNotification((notification) => {
      if (this.#session !== session) return;
      for (const listener of this.#notificationListeners) {
        try {
          listener(notification);
        } catch {
          // One host consumer cannot prevent delivery to other notification consumers.
        }
      }
    });
    this.#setState("initializing");
    try {
      await session.initialize();
    } catch (error) {
      this.#lastDiagnostics = session.diagnostics();
      if (this.#session === session) this.#session = undefined;
      if (this.#process === child) this.#process = undefined;
      this.#generation += 1;
      await session.close();
      throw error;
    }
    if (this.#stopping || this.#generation !== generation) {
      await session.close();
      throw new Error("App Server startup was superseded");
    }
    this.#setState("ready");
  }

  async #restartAfterCrash(): Promise<void> {
    while (!this.#stopping && this.#restartAttempts < this.#maxRestartAttempts) {
      const attempt = this.#restartAttempts++;
      this.#setState("restarting");
      await this.#wait(this.#restartDelay(attempt));
      if (this.#stopping) return;
      try {
        await this.#launch();
        return;
      } catch {
        this.#setState("crashed");
      }
    }
  }

  #restartDelay(attempt: number): number {
    return Math.min(
      this.#initialRestartDelayMs * 2 ** attempt,
      this.#maxRestartDelayMs,
    );
  }

  #setState(state: AppServerConnectionState): void {
    if (this.#state === state) return;
    this.#state = state;
    for (const listener of this.#stateListeners) {
      try {
        listener(state);
      } catch {
        // Connection state observers are isolated from supervisor lifecycle.
      }
    }
  }
}

function defaultSpawn(
  executable: string,
  args: readonly string[],
  options: SpawnAppServerOptions,
): ChildProcessWithoutNullStreams {
  return spawn(executable, [...args], {
    env: { ...options.environment },
    shell: false,
    stdio: "pipe",
  });
}

function wait(milliseconds: number): Promise<void> {
  return new Promise((resolve) => {
    const timeout = setTimeout(resolve, milliseconds);
    timeout.unref();
  });
}

function positiveInteger(value: number | undefined, fallback: number, name: string): number {
  const resolved = value ?? fallback;
  if (!Number.isSafeInteger(resolved) || resolved <= 0) {
    throw new Error(`${name} must be a positive safe integer`);
  }
  return resolved;
}

function nonNegativeInteger(
  value: number | undefined,
  fallback: number,
  name: string,
): number {
  const resolved = value ?? fallback;
  if (!Number.isSafeInteger(resolved) || resolved < 0) {
    throw new Error(`${name} must be a non-negative safe integer`);
  }
  return resolved;
}
