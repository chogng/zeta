import type { ModelListResult, SessionListResult, SessionResult, SessionSubscribeResult, SessionThreadReadResult, SessionThreadResult, SessionThreadSubscribeResult, TurnInteractionResolveResult, TurnInterruptResult, TurnStartResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IModelApi, ISessionApi, IThreadApi, ITurnApi } from "../common/sessionApi.js";

export function createSessionApi(): ISessionApi {
  return {
    create: (params) => invoke<SessionResult>("zeta:session:create", params),
    read: (params) => invoke<SessionResult>("zeta:session:read", params),
    list: () => invoke<SessionListResult>("zeta:session:list"),
    subscribe: (params) => invoke<SessionSubscribeResult>("zeta:session:subscribe", params),
    unsubscribe: (params) => invoke<void>("zeta:session:unsubscribe", params),
    createThread: (params) => invoke<SessionThreadResult>("zeta:session:thread:create", params),
    forkThread: (params) => invoke<SessionThreadResult>("zeta:session:thread:fork", params),
    archiveThread: (params) => invoke<SessionResult>("zeta:session:thread:archive", params),
    complete: (params) => invoke<SessionResult>("zeta:session:complete", params),
    archive: (params) => invoke<SessionResult>("zeta:session:archive", params),
    stop: (params) => invoke<SessionResult>("zeta:session:stop", params),
    setModel: (params) => invoke<SessionResult>("zeta:session:model:set", params),
  };
}

export function createModelApi(): IModelApi {
  return { list: () => invoke<ModelListResult>("zeta:model:list") };
}

export function createThreadApi(): IThreadApi {
  return {
    read: (params) => invoke<SessionThreadReadResult>("zeta:thread:read", params),
    subscribe: (params) => invoke<SessionThreadSubscribeResult>("zeta:thread:subscribe", params),
    unsubscribe: (params) => invoke<void>("zeta:thread:unsubscribe", params),
  };
}

export function createTurnApi(): ITurnApi {
  return {
    start: (params) => invoke<TurnStartResult>("zeta:turn:start", params),
    interrupt: (params) => invoke<TurnInterruptResult>("zeta:turn:interrupt", params),
    resolveInteraction: (params) => invoke<TurnInteractionResolveResult>("zeta:turn:interaction:resolve", params),
  };
}
