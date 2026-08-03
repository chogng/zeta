import type { ModelListResult, SessionCommandParams, SessionCreateParams, SessionListResult, SessionModelSetParams, SessionReadParams, SessionResult, SessionSubscribeParams, SessionSubscribeResult, SessionThreadArchiveParams, SessionThreadCreateParams, SessionThreadForkParams, SessionThreadResult, SessionUnsubscribeParams, ThreadReadParams, ThreadReadResult, ThreadSubscribeParams, ThreadSubscribeResult, ThreadUnsubscribeParams, TurnInteractionResolveParams, TurnInteractionResolveResult, TurnInterruptParams, TurnInterruptResult, TurnStartParams, TurnStartResult } from "../../../../../generated/app-server/types.js";

export interface ISessionApi {
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
  stop(params: SessionCommandParams): Promise<SessionResult>;
  setModel(params: SessionModelSetParams): Promise<SessionResult>;
}

export interface IModelApi {
  list(): Promise<ModelListResult>;
}

export interface IThreadApi {
  read(params: ThreadReadParams): Promise<ThreadReadResult>;
  subscribe(params: ThreadSubscribeParams): Promise<ThreadSubscribeResult>;
  unsubscribe(params: ThreadUnsubscribeParams): Promise<void>;
}

export interface ITurnApi {
  start(params: TurnStartParams): Promise<TurnStartResult>;
  interrupt(params: TurnInterruptParams): Promise<TurnInterruptResult>;
  resolveInteraction(params: TurnInteractionResolveParams): Promise<TurnInteractionResolveResult>;
}
