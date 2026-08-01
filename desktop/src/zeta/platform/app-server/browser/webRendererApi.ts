import { APP_SERVER_METHODS, type AppServerMethod, type MethodParams, type MethodResult } from "../../../../../generated/app-server/types.js";
import type { ZetaRendererApi } from "../common/renderer-api.js";
import { ViteDevAppServerConnection, type ViteDevAppServerConnectionOptions, type ViteDevAppServerMetadata, type ViteDevHotContext } from "./viteDevConnection.js";

export interface ConnectedWebRendererApi {
  readonly api: ZetaRendererApi;
  readonly metadata: ViteDevAppServerMetadata;
  dispose(): void;
}

/** Connects a browser Renderer API to the loopback Vite development bridge. */
export async function connectViteDevRendererApi(hot: ViteDevHotContext, options: ViteDevAppServerConnectionOptions = {}): Promise<ConnectedWebRendererApi> {
  const connection = new ViteDevAppServerConnection(hot, options);
  try {
    const metadata = await connection.connect();
    return {
      api: createRendererApi(connection),
      metadata,
      dispose: () => connection.dispose(),
    };
  } catch (error) {
    connection.dispose();
    throw error;
  }
}

function createRendererApi(connection: ViteDevAppServerConnection): ZetaRendererApi {
  const voidResult = <T>(promise: Promise<T>): Promise<void> => promise.then(() => undefined);
  const api: ZetaRendererApi = {
    appServer: {
      getConnectionState: () => Promise.resolve(connection.state),
      getSlashCommands: () => Promise.resolve(connection.slashCommands),
      onConnectionState: (listener) => connection.onStateChange(listener),
    },
    session: {
      create: (params) => request(connection, "session/create", params),
      read: (params) => request(connection, "session/read", params),
      list: () => request(connection, "session/list", {}),
      subscribe: (params) => request(connection, "session/subscribe", params),
      unsubscribe: (params) => voidResult(request(connection, "session/unsubscribe", params)),
      createThread: (params) => request(connection, "session/thread/create", params),
      forkThread: (params) => request(connection, "session/thread/fork", params),
      archiveThread: (params) => request(connection, "session/thread/archive", params),
      complete: (params) => request(connection, "session/complete", params),
      archive: (params) => request(connection, "session/archive", params),
      setModel: (params) => request(connection, "session/model/set", params),
    },
    model: {
      list: () => request(connection, "model/list", {}),
    },
    thread: {
      read: (params) => request(connection, "thread/read", params),
      subscribe: (params) => request(connection, "thread/subscribe", params),
      unsubscribe: (params) => voidResult(request(connection, "thread/unsubscribe", params)),
    },
    turn: {
      start: (params) => request(connection, "turn/start", params),
      interrupt: (params) => request(connection, "turn/interrupt", params),
      resolveInteraction: (params) => request(connection, "turn/interaction/resolve", params),
    },
    typst: {
      compile: (params) => request(connection, "document/typst/compile", params),
    },
    syntax: {
      open: (params) => request(connection, "document/syntax/open", params),
      change: (params) => request(connection, "document/syntax/change", params),
      close: (params) => voidResult(request(connection, "document/syntax/close", params)),
    },
    resource: {
      metadata: (params) => request(connection, "resource/metadata", params),
      read: (params) => request(connection, "resource/read", params),
      release: (params) => voidResult(request(connection, "resource/release", params)),
    },
    fs: {
      getMetadata: (params) => request(connection, "fs/getMetadata", params),
      readDirectory: (params) => request(connection, "fs/readDirectory", params),
      readFile: (params) => request(connection, "fs/readFile", params),
    },
    git: {
      status: () => request(connection, "git/status", {}),
      stage: (params) => request(connection, "git/stage", params),
      unstage: (params) => request(connection, "git/unstage", params),
      discardWorktree: (params) => request(connection, "git/discardWorktree", params),
      commit: (params) => request(connection, "git/commit", params),
      fetch: () => request(connection, "git/fetch", {}),
      pull: () => request(connection, "git/pull", {}),
      push: () => request(connection, "git/push", {}),
    },
    workspaceSearch: {
      start: (params) => request(connection, "workspace/search/start", params),
      read: (params) => request(connection, "workspace/search/read", params),
      cancel: (params) => voidResult(request(connection, "workspace/search/cancel", params)),
    },
    terminal: {
      listProfiles: () => request(connection, "terminal/profile/list", {}),
      create: (params) => request(connection, "terminal/create", params),
      write: (params) => voidResult(request(connection, "terminal/write", params)),
      resize: (params) => voidResult(request(connection, "terminal/resize", params)),
      read: (params) => request(connection, "terminal/read", params),
      close: (params) => voidResult(request(connection, "terminal/close", params)),
    },
    events: {
      subscribe: (listener) => connection.onNotification(listener),
    },
  };
  return api;
}

function request<M extends AppServerMethod>(connection: ViteDevAppServerConnection, method: M, params: MethodParams<M>): Promise<MethodResult<M>> {
  return connection.request(APP_SERVER_METHODS[method], params);
}
