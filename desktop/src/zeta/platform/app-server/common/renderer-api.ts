import type {
  FsGetMetadataParams,
  FsGetMetadataResult,
  FsReadDirectoryParams,
  FsReadDirectoryResult,
  FsReadFileParams,
  FsReadFileResult,
  ResourceMetadataParams,
  ResourceMetadataResult,
  ResourceReadParams,
  ResourceReadResult,
  ResourceReleaseParams,
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
  TurnInteractionResolveParams,
  TurnInteractionResolveResult,
  TurnStartParams,
  TurnStartResult,
  TypstCompileParams,
  TypstCompileResult,
  WorkspaceSearchCancelParams,
  WorkspaceSearchReadParams,
  WorkspaceSearchReadResult,
  WorkspaceSearchStartParams,
  WorkspaceSearchStartResult,
} from "../../../../../generated/app-server/types.js";

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
    resolveInteraction(
      params: TurnInteractionResolveParams,
    ): Promise<TurnInteractionResolveResult>;
  };
  typst: {
    compile(params: TypstCompileParams): Promise<TypstCompileResult>;
  };
  resource: {
    metadata(params: ResourceMetadataParams): Promise<ResourceMetadataResult>;
    read(params: ResourceReadParams): Promise<ResourceReadResult>;
    release(params: ResourceReleaseParams): Promise<void>;
  };
  fs: {
    getMetadata(params: FsGetMetadataParams): Promise<FsGetMetadataResult>;
    readDirectory(params: FsReadDirectoryParams): Promise<FsReadDirectoryResult>;
    readFile(params: FsReadFileParams): Promise<FsReadFileResult>;
  };
  workspaceSearch: {
    start(
      params: WorkspaceSearchStartParams,
    ): Promise<WorkspaceSearchStartResult>;
    read(
      params: WorkspaceSearchReadParams,
    ): Promise<WorkspaceSearchReadResult>;
    cancel(params: WorkspaceSearchCancelParams): Promise<void>;
  };
  events: {
    subscribe(listener: (event: ServerNotification) => void): DisposableHandle;
  };
}
