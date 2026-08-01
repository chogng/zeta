import type { ZetaRendererApi } from "../common/renderer-api.js";

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
      getSlashCommands: () => unavailable("appServer.getSlashCommands"),
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
      setModel: () => unavailable("session.setModel"),
    },
    model: {
      list: () => unavailable("model.list"),
    },
    thread: {
      read: () => unavailable("thread.read"),
      subscribe: () => unavailable("thread.subscribe"),
      unsubscribe: () => unavailable("thread.unsubscribe"),
    },
    turn: {
      start: () => unavailable("turn.start"),
      interrupt: () => unavailable("turn.interrupt"),
      resolveInteraction: () =>
        unavailable("turn.resolveInteraction"),
    },
    typst: {
      compile: () => unavailable("typst.compile"),
    },
    syntax: {
      open: () => unavailable("syntax.open"),
      change: () => unavailable("syntax.change"),
      close: () => unavailable("syntax.close"),
    },
    resource: {
      metadata: () => unavailable("resource.metadata"),
      read: () => unavailable("resource.read"),
      release: () => unavailable("resource.release"),
    },
    fs: {
      getMetadata: () => unavailable("fs.getMetadata"),
      readDirectory: () => unavailable("fs.readDirectory"),
      readFile: () => unavailable("fs.readFile"),
    },
    git: {
      status: () => unavailable("git.status"),
      history: () => unavailable("git.history"),
      stage: () => unavailable("git.stage"),
      unstage: () => unavailable("git.unstage"),
      discardWorktree: () => unavailable("git.discardWorktree"),
      commit: () => unavailable("git.commit"),
      fetch: () => unavailable("git.fetch"),
      pull: () => unavailable("git.pull"),
      push: () => unavailable("git.push"),
    },
    workspaceSearch: {
      start: () => unavailable("workspaceSearch.start"),
      read: () => unavailable("workspaceSearch.read"),
      cancel: () => unavailable("workspaceSearch.cancel"),
    },
    terminal: {
      listProfiles: () => unavailable("terminal.listProfiles"),
      create: () => unavailable("terminal.create"),
      write: () => unavailable("terminal.write"),
      resize: () => unavailable("terminal.resize"),
      read: () => unavailable("terminal.read"),
      close: () => unavailable("terminal.close"),
    },
    events: {
      subscribe: inertSubscription,
    },
  };
}
