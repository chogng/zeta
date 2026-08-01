import { APP_SERVER_METHODS, type SessionCommandParams, type SessionCreateParams, type SessionModelSetParams, type SessionReadParams, type SessionSubscribeParams, type SessionThreadArchiveParams, type SessionThreadCreateParams, type SessionThreadForkParams, type SessionUnsubscribeParams, type ThreadReadParams, type ThreadSubscribeParams, type ThreadUnsubscribeParams, type TurnInteractionResolveParams, type TurnInterruptParams, type TurnStartParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { boolean, nonEmptyString, nonNegativeInteger, record, string, stringEnum } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

/** Exact-shape IPC routes for Session, Thread, Turn, and model operations. */
export function sessionIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({
      channel: "zeta:session:create",
      validate: sessionCreateParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["session/create"], params),
    }),
    route({
      channel: "zeta:session:read",
      validate: sessionReadParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["session/read"], params),
    }),
    route({
      channel: "zeta:session:list",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["session/list"], {}),
    }),
    route({
      channel: "zeta:session:subscribe",
      validate: sessionSubscribeParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["session/subscribe"], params),
    }),
    route({
      channel: "zeta:session:unsubscribe",
      validate: sessionUnsubscribeParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["session/unsubscribe"], params),
    }),
    route({
      channel: "zeta:session:thread:create",
      validate: sessionThreadCreateParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["session/thread/create"], params),
    }),
    route({
      channel: "zeta:session:thread:fork",
      validate: sessionThreadForkParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["session/thread/fork"], params),
    }),
    route({
      channel: "zeta:session:thread:archive",
      validate: sessionThreadArchiveParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["session/thread/archive"], params),
    }),
    route({
      channel: "zeta:session:complete",
      validate: sessionCommandParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["session/complete"], params),
    }),
    route({
      channel: "zeta:session:archive",
      validate: sessionCommandParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["session/archive"], params),
    }),
    route({
      channel: "zeta:session:model:set",
      validate: sessionModelSetParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["session/model/set"], params),
    }),
    route({
      channel: "zeta:model:list",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["model/list"], {}),
    }),
    route({
      channel: "zeta:thread:read",
      validate: threadReadParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["thread/read"], params),
    }),
    route({
      channel: "zeta:thread:subscribe",
      validate: threadSubscribeParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["thread/subscribe"], params),
    }),
    route({
      channel: "zeta:thread:unsubscribe",
      validate: threadUnsubscribeParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["thread/unsubscribe"], params),
    }),
    route({
      channel: "zeta:turn:start",
      validate: turnStartParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["turn/start"], params),
    }),
    route({
      channel: "zeta:turn:interrupt",
      validate: turnInterruptParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["turn/interrupt"], params),
    }),
    route({
      channel: "zeta:turn:interaction:resolve",
      validate: turnInteractionResolveParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["turn/interaction/resolve"], params),
    }),
  ];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
  return {
    channel: definition.channel,
    validate: definition.validate,
    invoke: (params) => definition.invoke(params as P),
  };
}

function emptyParams(value: unknown): Record<string, never> {
  if (value === undefined) return {};
  return record(value, []) as Record<string, never>;
}

function sessionCreateParams(value: unknown): SessionCreateParams {
  const params = record(value, ["commandId", "title"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    title: string(params.title, "title"),
  };
}

function sessionReadParams(value: unknown): SessionReadParams {
  const params = record(value, ["sessionId"]);
  return { sessionId: nonEmptyString(params.sessionId, "sessionId") };
}

function sessionSubscribeParams(value: unknown): SessionSubscribeParams {
  const params = record(value, ["sessionId", "afterSequence"]);
  return {
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    afterSequence: nonNegativeInteger(params.afterSequence, "afterSequence"),
  };
}

function sessionUnsubscribeParams(value: unknown): SessionUnsubscribeParams {
  return sessionReadParams(value);
}

function sessionCommandParams(value: unknown): SessionCommandParams {
  const params = record(value, ["commandId", "sessionId", "expectedSequence"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
  };
}

function sessionModelSetParams(value: unknown): SessionModelSetParams {
  const params = record(value, ["commandId", "sessionId", "expectedSequence", "model"]);
  const model = record(params.model, ["provider", "model"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
    model: {
      provider: nonEmptyString(model.provider, "model.provider"),
      model: nonEmptyString(model.model, "model.model"),
    },
  };
}

function sessionThreadCreateParams(value: unknown): SessionThreadCreateParams {
  const params = record(value, ["commandId", "sessionId", "expectedSequence", "title"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
    title: string(params.title, "title"),
  };
}

function sessionThreadForkParams(value: unknown): SessionThreadForkParams {
  const params = record(value, ["commandId", "sessionId", "expectedSequence", "parentThreadId", "title"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
    parentThreadId: nonEmptyString(params.parentThreadId, "parentThreadId"),
    title: string(params.title, "title"),
  };
}

function sessionThreadArchiveParams(value: unknown): SessionThreadArchiveParams {
  const params = record(value, ["commandId", "sessionId", "expectedSequence", "threadId"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
    threadId: nonEmptyString(params.threadId, "threadId"),
  };
}

function threadReadParams(value: unknown): ThreadReadParams {
  const params = record(value, ["threadId"]);
  return { threadId: nonEmptyString(params.threadId, "threadId") };
}

function threadSubscribeParams(value: unknown): ThreadSubscribeParams {
  const params = record(value, ["threadId", "afterSequence"]);
  return {
    threadId: nonEmptyString(params.threadId, "threadId"),
    afterSequence: nonNegativeInteger(params.afterSequence, "afterSequence"),
  };
}

function threadUnsubscribeParams(value: unknown): ThreadUnsubscribeParams {
  return threadReadParams(value);
}

function turnStartParams(value: unknown): TurnStartParams {
  const params = record(value, ["commandId", "sessionId", "threadId", "expectedSequence", "input"]);
  if (!Array.isArray(params.input) || params.input.length === 0) {
    throw new Error("input must be a non-empty array");
  }
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    threadId: nonEmptyString(params.threadId, "threadId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
    input: params.input.map((value, index) => {
      const item = record(value, ["type", "text"]);
      if (item.type !== "text") throw new Error(`input[${index}].type must be text`);
      return { type: "text" as const, text: nonEmptyString(item.text, `input[${index}].text`) };
    }),
  };
}

function turnInterruptParams(value: unknown): TurnInterruptParams {
  const params = record(value, ["commandId", "sessionId", "threadId", "turnId", "expectedSequence"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    threadId: nonEmptyString(params.threadId, "threadId"),
    turnId: nonEmptyString(params.turnId, "turnId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
  };
}

function turnInteractionResolveParams(value: unknown): TurnInteractionResolveParams {
  const params = record(value, ["commandId", "sessionId", "threadId", "turnId", "requestId", "expectedSequence", "response"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    threadId: nonEmptyString(params.threadId, "threadId"),
    turnId: nonEmptyString(params.turnId, "turnId"),
    requestId: nonEmptyString(params.requestId, "requestId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
    response: agentResponse(params.response),
  };
}

function agentResponse(value: unknown): TurnInteractionResolveParams["response"] {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("response must be an object");
  }
  const type = (value as Record<string, unknown>).type;
  switch (type) {
    case "approval": {
      const response = record(value, ["type", "response"]);
      const decision = record(response.response, ["decision"]);
      return {
        type,
        response: { decision: stringEnum(decision.decision, "response.decision", ["approveOnce", "decline"] as const) },
      };
    }
    case "userInput": {
      const response = record(value, ["type", "response"]);
      const payload = record(response.response, ["answers"]);
      if (typeof payload.answers !== "object" || payload.answers === null || Array.isArray(payload.answers)) {
        throw new Error("response.answers must be an object");
      }
      const answers: Record<string, { value: string }> = {};
      for (const [id, answer] of Object.entries(payload.answers as Record<string, unknown>)) {
        if (!id) throw new Error("response answer ID must not be empty");
        const item = record(answer, ["value"]);
        answers[id] = { value: string(item.value, `response.answers.${id}.value`) };
      }
      return { type, response: { answers } };
    }
    case "dynamicTool": {
      const response = record(value, ["type", "response"]);
      const payload = record(response.response, ["callId", "content", "success"]);
      if (!Array.isArray(payload.content)) throw new Error("response.content must be an array");
      return {
        type,
        response: {
          callId: nonEmptyString(payload.callId, "response.callId"),
          content: payload.content.map((entry, index) => {
            if (typeof entry !== "object" || entry === null || Array.isArray(entry)) {
              throw new Error(`response.content[${index}] must be an object`);
            }
            const entryType = (entry as Record<string, unknown>).type;
            if (entryType === "text") {
              const text = record(entry, ["type", "text"]);
              return { type: "text" as const, text: string(text.text, `response.content[${index}].text`) };
            }
            if (entryType === "image") {
              const image = record(entry, ["type", "dataUrl"]);
              return { type: "image" as const, dataUrl: nonEmptyString(image.dataUrl, `response.content[${index}].dataUrl`) };
            }
            throw new Error(`response.content[${index}].type is unsupported`);
          }),
          success: boolean(payload.success, "response.success"),
        },
      };
    }
    default:
      throw new Error("response.type is unsupported");
  }
}
