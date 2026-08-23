import type { ModelListResult, SessionCreateParams, SessionListResult, SessionReadParams, SessionRequest, SessionRequestParams, SessionRequestResult, SessionResult, SessionSubscribeParams, SessionSubscribeResult, SessionThreadReadParams, SessionThreadReadResult, SessionThreadResult, SessionThreadSubscribeParams, SessionThreadSubscribeResult, SessionThreadUnsubscribeParams, SessionUnsubscribeParams, TurnInteractionResolveResult, TurnInterruptResult, TurnStartResult } from "../../../../../generated/app-server/types.js";

export type { SessionRequestResult };

export type SessionMutationParams = Omit<SessionRequestParams, "request">;
export type SessionOperationInput<T extends SessionRequest["type"]> = SessionMutationParams & Omit<Extract<SessionRequest, { type: T }>, "type">;

export function sessionRequest(params: SessionMutationParams, request: SessionRequest): SessionRequestParams {
	return { ...params, request };
}

export function sessionResult(result: SessionRequestResult): SessionResult {
	if (result.type !== "session") throw new Error(`Expected Session result, received ${result.type}.`);
	return result.value;
}

export function sessionThreadResult(result: SessionRequestResult): SessionThreadResult {
	if (result.type !== "thread") throw new Error(`Expected Thread result, received ${result.type}.`);
	return result.value;
}

export function turnStartResult(result: SessionRequestResult): TurnStartResult {
	if (result.type !== "turn") throw new Error(`Expected Turn result, received ${result.type}.`);
	return result.value;
}

export function turnInterruptResult(result: SessionRequestResult): TurnInterruptResult {
	if (result.type !== "turnInterrupt") throw new Error(`Expected Turn interrupt result, received ${result.type}.`);
	return result.value;
}

export function turnInteractionResolveResult(result: SessionRequestResult): TurnInteractionResolveResult {
	if (result.type !== "interaction") throw new Error(`Expected interaction result, received ${result.type}.`);
	return result.value;
}

export interface ISessionApi {
	create(params: SessionCreateParams): Promise<SessionResult>;
	read(params: SessionReadParams): Promise<SessionResult>;
	list(): Promise<SessionListResult>;
	subscribe(params: SessionSubscribeParams): Promise<SessionSubscribeResult>;
	unsubscribe(params: SessionUnsubscribeParams): Promise<void>;
	createThread(params: SessionOperationInput<"createThread">): Promise<SessionThreadResult>;
	forkThread(params: SessionOperationInput<"forkThread">): Promise<SessionThreadResult>;
	archiveThread(params: SessionOperationInput<"archiveThread">): Promise<SessionResult>;
	complete(params: SessionOperationInput<"complete">): Promise<SessionResult>;
	archive(params: SessionOperationInput<"archive">): Promise<SessionResult>;
	stop(params: SessionOperationInput<"stop">): Promise<SessionResult>;
	setModel(params: SessionOperationInput<"setModel">): Promise<SessionResult>;
}

export interface IModelApi {
	list(): Promise<ModelListResult>;
}

export interface IThreadApi {
	read(params: SessionThreadReadParams): Promise<SessionThreadReadResult>;
	subscribe(params: SessionThreadSubscribeParams): Promise<SessionThreadSubscribeResult>;
	unsubscribe(params: SessionThreadUnsubscribeParams): Promise<void>;
}

export interface ITurnApi {
	start(params: SessionOperationInput<"startTurn">): Promise<TurnStartResult>;
	interrupt(params: SessionOperationInput<"interruptTurn">): Promise<TurnInterruptResult>;
	resolveInteraction(params: SessionOperationInput<"resolveInteraction">): Promise<TurnInteractionResolveResult>;
}
