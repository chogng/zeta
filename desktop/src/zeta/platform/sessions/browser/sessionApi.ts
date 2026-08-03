import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IModelApi, ISessionApi, IThreadApi, ITurnApi } from "../common/sessionApi.js";

export function createDisconnectedSessionApi(unavailable: UnavailableOperation): ISessionApi {
  return {
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
    stop: () => unavailable("session.stop"),
    setModel: () => unavailable("session.setModel"),
  };
}

export function createDisconnectedModelApi(unavailable: UnavailableOperation): IModelApi {
  return { list: () => unavailable("model.list") };
}

export function createDisconnectedThreadApi(unavailable: UnavailableOperation): IThreadApi {
  return {
    read: () => unavailable("thread.read"),
    subscribe: () => unavailable("thread.subscribe"),
    unsubscribe: () => unavailable("thread.unsubscribe"),
  };
}

export function createDisconnectedTurnApi(unavailable: UnavailableOperation): ITurnApi {
  return {
    start: () => unavailable("turn.start"),
    interrupt: () => unavailable("turn.interrupt"),
    resolveInteraction: () => unavailable("turn.resolveInteraction"),
  };
}

export function createViteDevSessionApi(connection: ViteDevAppServerConnection): ISessionApi {
  return {
    create: (params) => viteDevRequest(connection, "session/create", params),
    read: (params) => viteDevRequest(connection, "session/read", params),
    list: () => viteDevRequest(connection, "session/list", {}),
    subscribe: (params) => viteDevRequest(connection, "session/subscribe", params),
    unsubscribe: (params) => voidResult(viteDevRequest(connection, "session/unsubscribe", params)),
    createThread: (params) => viteDevRequest(connection, "session/thread/create", params),
    forkThread: (params) => viteDevRequest(connection, "session/thread/fork", params),
    archiveThread: (params) => viteDevRequest(connection, "session/thread/archive", params),
    complete: (params) => viteDevRequest(connection, "session/complete", params),
    archive: (params) => viteDevRequest(connection, "session/archive", params),
    stop: (params) => viteDevRequest(connection, "session/stop", params),
    setModel: (params) => viteDevRequest(connection, "session/model/set", params),
  };
}

export function createViteDevModelApi(connection: ViteDevAppServerConnection): IModelApi {
  return { list: () => viteDevRequest(connection, "model/list", {}) };
}

export function createViteDevThreadApi(connection: ViteDevAppServerConnection): IThreadApi {
  return {
    read: (params) => viteDevRequest(connection, "thread/read", params),
    subscribe: (params) => viteDevRequest(connection, "thread/subscribe", params),
    unsubscribe: (params) => voidResult(viteDevRequest(connection, "thread/unsubscribe", params)),
  };
}

export function createViteDevTurnApi(connection: ViteDevAppServerConnection): ITurnApi {
  return {
    start: (params) => viteDevRequest(connection, "turn/start", params),
    interrupt: (params) => viteDevRequest(connection, "turn/interrupt", params),
    resolveInteraction: (params) => viteDevRequest(connection, "turn/interaction/resolve", params),
  };
}
