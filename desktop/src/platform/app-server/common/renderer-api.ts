import type {
  ServerNotification,
  SessionCommandParams,
  SessionCreateParams,
  SessionListResult,
  SessionReadParams,
  SessionResult,
  SessionSubscribeParams,
  SessionSubscribeResult,
  SessionThreadArchiveParams,
  SessionThreadCreateParams,
  SessionThreadForkParams,
  SessionThreadResult,
  SessionUnsubscribeParams,
  ThreadReadParams,
  ThreadReadResult,
  ThreadSubscribeParams,
  ThreadSubscribeResult,
  ThreadUnsubscribeParams,
  TurnInterruptParams,
  TurnInterruptResult,
  TurnStartParams,
  TurnStartResult,
} from "../../../../generated/app-server/types.js";

/**
 * A string-keyed cleanup handle that can cross Electron's contextBridge.
 *
 * Symbol-keyed standard disposal methods cannot cross that serialization
 * boundary. Renderer code should adapt this handle to a local `IDisposable`
 * when it needs to transfer ownership.
 */
export interface DisposableHandle {
  dispose(): void;
}

export type AppServerConnectionState =
  | "stopped"
  | "starting"
  | "initializing"
  | "ready"
  | "stopping"
  | "crashed"
  | "restarting";

/**
 * The narrowly scoped, typed capability bridge available to the renderer.
 *
 * Electron preload implementations expose this contract without leaking IPC or
 * other Electron primitives to workbench code.
 */
export interface ZetaRendererApi {
  appServer: {
    getConnectionState(): Promise<AppServerConnectionState>;
    onConnectionState(
      listener: (state: AppServerConnectionState) => void,
    ): DisposableHandle;
  };
  session: {
    create(params: SessionCreateParams): Promise<SessionResult>;
    read(params: SessionReadParams): Promise<SessionResult>;
    list(): Promise<SessionListResult>;
    subscribe(params: SessionSubscribeParams): Promise<SessionSubscribeResult>;
    unsubscribe(params: SessionUnsubscribeParams): Promise<void>;
    createThread(params: SessionThreadCreateParams): Promise<SessionThreadResult>;
    forkThread(params: SessionThreadForkParams): Promise<SessionThreadResult>;
    archiveThread(params: SessionThreadArchiveParams): Promise<SessionResult>;
    complete(params: SessionCommandParams): Promise<SessionResult>;
    archive(params: SessionCommandParams): Promise<SessionResult>;
  };
  thread: {
    read(params: ThreadReadParams): Promise<ThreadReadResult>;
    subscribe(params: ThreadSubscribeParams): Promise<ThreadSubscribeResult>;
    unsubscribe(params: ThreadUnsubscribeParams): Promise<void>;
  };
  turn: {
    start(params: TurnStartParams): Promise<TurnStartResult>;
    interrupt(params: TurnInterruptParams): Promise<TurnInterruptResult>;
  };
  events: {
    subscribe(listener: (event: ServerNotification) => void): DisposableHandle;
  };
}
