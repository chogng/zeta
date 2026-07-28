import type {
  ZetaRendererApi,
} from "../common/renderer-api.js";

/**
 * Error returned when a standalone Web Workbench has no App Server host.
 */
export class WebAppServerUnavailableError extends Error {
  constructor(readonly operation: string) {
    super(
      `Web App Server operation '${operation}' is unavailable because ` +
        "no Web host API was provided",
    );
    this.name = "WebAppServerUnavailableError";
  }
}

/**
 * Creates the explicit disconnected renderer capability used by standalone
 * Web pages until an embedder supplies a remote App Server implementation.
 */
export function createDisconnectedRendererApi(): ZetaRendererApi {
  const unavailable = <T>(operation: string): Promise<T> =>
    Promise.reject(new WebAppServerUnavailableError(operation));
  const inertSubscription = () => ({ dispose(): void {} });

  return {
    appServer: {
      getConnectionState: () => Promise.resolve("stopped"),
      onConnectionState: inertSubscription,
    },
    session: {
      create: () => unavailable("session.create"),
      read: () => unavailable("session.read"),
      list: () => unavailable("session.list"),
      subscribe: () => unavailable("session.subscribe"),
      unsubscribe: () => unavailable("session.unsubscribe"),
      createThread: () => unavailable("session.createThread"),
      forkThread: () => unavailable("session.forkThread"),
      archiveThread: () => unavailable("session.archiveThread"),
      complete: () => unavailable("session.complete"),
      archive: () => unavailable("session.archive"),
    },
    thread: {
      read: () => unavailable("thread.read"),
      subscribe: () => unavailable("thread.subscribe"),
      unsubscribe: () => unavailable("thread.unsubscribe"),
    },
    turn: {
      start: () => unavailable("turn.start"),
      interrupt: () => unavailable("turn.interrupt"),
    },
    typst: {
      compile: () => unavailable("typst.compile"),
    },
    resource: {
      metadata: () => unavailable("resource.metadata"),
      read: () => unavailable("resource.read"),
      release: () => unavailable("resource.release"),
    },
    fs: {
      getMetadata: () => unavailable("fs.getMetadata"),
      readDirectory: () => unavailable("fs.readDirectory"),
    },
    events: {
      subscribe: inertSubscription,
    },
  };
}
