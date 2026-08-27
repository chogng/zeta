import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import { sessionRequest, sessionResult, sessionThreadResult, turnInteractionResolveResult, turnInterruptResult, turnStartResult, turnSteerResult } from "../common/sessionApi.js";
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
		getGoal: () => unavailable("thread.goal.get"),
		setGoal: () => unavailable("thread.goal.set"),
		clearGoal: () => unavailable("thread.goal.clear"),
	};
}

export function createDisconnectedTurnApi(unavailable: UnavailableOperation): ITurnApi {
	return {
		start: () => unavailable("turn.start"),
		compact: () => unavailable("turn.compact"),
		steer: () => unavailable("turn.steer"),
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
		createThread: (params) => viteDevRequest(connection, "session/request", sessionRequest(params, { type: "createThread", title: params.title })).then(sessionThreadResult),
		forkThread: (params) => viteDevRequest(connection, "session/request", sessionRequest(params, { type: "forkThread", parentThreadId: params.parentThreadId, title: params.title })).then(sessionThreadResult),
		archiveThread: (params) => viteDevRequest(connection, "session/request", sessionRequest(params, { type: "archiveThread", threadId: params.threadId })).then(sessionResult),
		complete: (params) => viteDevRequest(connection, "session/request", sessionRequest(params, { type: "complete" })).then(sessionResult),
		archive: (params) => viteDevRequest(connection, "session/request", sessionRequest(params, { type: "archive" })).then(sessionResult),
		stop: (params) => viteDevRequest(connection, "session/request", sessionRequest(params, { type: "stop" })).then(sessionResult),
		setModel: (params) => viteDevRequest(connection, "session/request", sessionRequest(params, { type: "setModel", model: params.model })).then(sessionResult),
	};
}

export function createViteDevModelApi(connection: ViteDevAppServerConnection): IModelApi {
	return { list: () => viteDevRequest(connection, "model/list", {}) };
}

export function createViteDevThreadApi(connection: ViteDevAppServerConnection): IThreadApi {
	return {
		read: (params) => viteDevRequest(connection, "session/thread/read", params),
		subscribe: (params) => viteDevRequest(connection, "session/thread/subscribe", params),
		unsubscribe: (params) => voidResult(viteDevRequest(connection, "session/thread/unsubscribe", params)),
		getGoal: (params) => viteDevRequest(connection, "thread/goal/get", params),
		setGoal: (params) => viteDevRequest(connection, "thread/goal/set", params),
		clearGoal: (params) => viteDevRequest(connection, "thread/goal/clear", params),
	};
}

export function createViteDevTurnApi(connection: ViteDevAppServerConnection): ITurnApi {
	return {
		start: (params) => viteDevRequest(connection, "session/request", sessionRequest({ commandId: params.commandId, sessionId: params.sessionId, expectedSequence: params.expectedSequence }, { type: "startTurn", threadId: params.threadId, approvalMode: params.approvalMode, input: params.input })).then(turnStartResult),
		compact: (params) => viteDevRequest(connection, "session/request", sessionRequest({ commandId: params.commandId, sessionId: params.sessionId, expectedSequence: params.expectedSequence }, { type: "compactContext", threadId: params.threadId, retentionPrompt: params.retentionPrompt })).then(turnStartResult),
		steer: (params) => viteDevRequest(connection, "session/request", sessionRequest({ commandId: params.commandId, sessionId: params.sessionId, expectedSequence: params.expectedSequence }, { type: "steerTurn", threadId: params.threadId, turnId: params.turnId, input: params.input })).then(turnSteerResult),
		interrupt: (params) => viteDevRequest(connection, "session/request", sessionRequest(params, { type: "interruptTurn", threadId: params.threadId, turnId: params.turnId })).then(turnInterruptResult),
		resolveInteraction: (params) => viteDevRequest(connection, "session/request", sessionRequest(params, { type: "resolveInteraction", threadId: params.threadId, turnId: params.turnId, requestId: params.requestId, response: params.response })).then(turnInteractionResolveResult),
	};
}
