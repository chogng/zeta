import type { ModelListResult, ModelRef, SessionListResult, SessionResult, SessionSubscribeResult, SessionThreadReadResult, SessionThreadResult, SessionThreadSubscribeResult, ThreadGoalClearResponse, ThreadGoalGetResponse, ThreadGoalSetResponse, TurnInteractionResolveResult, TurnInterruptResult, TurnStartResult, TurnSteerResult } from "../../../../../generated/app-server/types.js";
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
		archive: (params) => invoke<SessionResult>("zeta:session:archive", params),
		stop: (params) => invoke<SessionResult>("zeta:session:stop", params),
	};
}

export function createModelApi(): IModelApi {
	return {
		list: () => invoke<ModelListResult>("zeta:model:list"),
		readPreferred: () => invoke<ModelRef | null>("zeta:model:preferred:read"),
		setPreferred: (params) => invoke<void>("zeta:model:preferred:set", params),
	};
}

export function createThreadApi(): IThreadApi {
	return {
		read: (params) => invoke<SessionThreadReadResult>("zeta:thread:read", params),
		subscribe: (params) => invoke<SessionThreadSubscribeResult>("zeta:thread:subscribe", params),
		unsubscribe: (params) => invoke<void>("zeta:thread:unsubscribe", params),
		getGoal: (params) => invoke<ThreadGoalGetResponse>("zeta:thread:goal:get", params),
		setGoal: (params) => invoke<ThreadGoalSetResponse>("zeta:thread:goal:set", params),
		clearGoal: (params) => invoke<ThreadGoalClearResponse>("zeta:thread:goal:clear", params),
	};
}

export function createTurnApi(): ITurnApi {
	return {
		start: (params) => invoke<TurnStartResult>("zeta:turn:start", params),
		compact: (params) => invoke<TurnStartResult>("zeta:turn:compact", params),
		steer: (params) => invoke<TurnSteerResult>("zeta:turn:steer", params),
		interrupt: (params) => invoke<TurnInterruptResult>("zeta:turn:interrupt", params),
		resolveInteraction: (params) => invoke<TurnInteractionResolveResult>("zeta:turn:interaction:resolve", params),
	};
}
