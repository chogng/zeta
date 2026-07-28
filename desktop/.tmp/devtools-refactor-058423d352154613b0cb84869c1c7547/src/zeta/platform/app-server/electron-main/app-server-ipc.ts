import type {
  ResourceMetadataParams,
  ResourceReadParams,
  ResourceReleaseParams,
  SessionCommandParams,
  SessionCreateParams,
  SessionReadParams,
  SessionSubscribeParams,
  SessionThreadArchiveParams,
  SessionThreadCreateParams,
  SessionThreadForkParams,
  SessionUnsubscribeParams,
  ThreadReadParams,
  ThreadSubscribeParams,
  ThreadUnsubscribeParams,
  TurnInterruptParams,
  TurnStartParams,
  TypstCompileParams,
} from "../../../../generated/app-server/types.js";
import { APP_SERVER_METHODS } from "../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "./app-server-supervisor.js";
import type { IpcRoute } from "./trusted-ipc-router.js";

export function appServerIpcRoutes(
  supervisor: AppServerSupervisor,
): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({
      channel: "zeta:app-server:state",
      validate: emptyParams,
      invoke: () => supervisor.state,
    }),
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
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["session/unsubscribe"], params),
    }),
    route({
      channel: "zeta:session:thread:create",
      validate: sessionThreadCreateParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["session/thread/create"], params),
    }),
    route({
      channel: "zeta:session:thread:fork",
      validate: sessionThreadForkParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["session/thread/fork"], params),
    }),
    route({
      channel: "zeta:session:thread:archive",
      validate: sessionThreadArchiveParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["session/thread/archive"], params),
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
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["thread/unsubscribe"], params),
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
      channel: "zeta:typst:compile",
      validate: typstCompileParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["document/typst/compile"], params),
    }),
    route({
      channel: "zeta:resource:metadata",
      validate: resourceMetadataParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["resource/metadata"], params),
    }),
    route({
      channel: "zeta:resource:read",
      validate: resourceReadParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["resource/read"], params),
    }),
    route({
      channel: "zeta:resource:release",
      validate: resourceReleaseParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["resource/release"], params),
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

function sessionThreadCreateParams(value: unknown): SessionThreadCreateParams {
  const params = record(value, [
    "commandId",
    "sessionId",
    "expectedSequence",
    "title",
  ]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
    title: string(params.title, "title"),
  };
}

function sessionThreadForkParams(value: unknown): SessionThreadForkParams {
  const params = record(value, [
    "commandId",
    "sessionId",
    "expectedSequence",
    "parentThreadId",
    "title",
  ]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
    parentThreadId: nonEmptyString(params.parentThreadId, "parentThreadId"),
    title: string(params.title, "title"),
  };
}

function sessionThreadArchiveParams(value: unknown): SessionThreadArchiveParams {
  const params = record(value, [
    "commandId",
    "sessionId",
    "expectedSequence",
    "threadId",
  ]);
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
  const params = record(value, [
    "commandId",
    "sessionId",
    "threadId",
    "expectedSequence",
    "input",
  ]);
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
      if (item.type !== "text") {
        throw new Error(`input[${index}].type must be text`);
      }
      return {
        type: "text" as const,
        text: nonEmptyString(item.text, `input[${index}].text`),
      };
    }),
  };
}

function turnInterruptParams(value: unknown): TurnInterruptParams {
  const params = record(value, [
    "commandId",
    "sessionId",
    "threadId",
    "turnId",
    "expectedSequence",
  ]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    threadId: nonEmptyString(params.threadId, "threadId"),
    turnId: nonEmptyString(params.turnId, "turnId"),
    expectedSequence: nonNegativeInteger(params.expectedSequence, "expectedSequence"),
  };
}

const MAX_TYPST_SOURCE_BYTES = 1024 * 1024;
const MAX_RESOURCE_READ_BYTES = 262_144;

function typstCompileParams(value: unknown): TypstCompileParams {
  const params = record(value, ["source"]);
  const source = string(params.source, "source");
  if (new TextEncoder().encode(source).byteLength > MAX_TYPST_SOURCE_BYTES) {
    throw new Error(
      `source must not exceed ${MAX_TYPST_SOURCE_BYTES} UTF-8 bytes`,
    );
  }
  return { source };
}

function resourceMetadataParams(value: unknown): ResourceMetadataParams {
  const params = record(value, ["resourceId"]);
  return { resourceId: nonEmptyString(params.resourceId, "resourceId") };
}

function resourceReleaseParams(value: unknown): ResourceReleaseParams {
  return resourceMetadataParams(value);
}

function resourceReadParams(value: unknown): ResourceReadParams {
  const params = record(value, ["resourceId", "offset", "maxBytes"]);
  const maxBytes = positiveInteger(params.maxBytes, "maxBytes");
  if (maxBytes > MAX_RESOURCE_READ_BYTES) {
    throw new Error(`maxBytes must not exceed ${MAX_RESOURCE_READ_BYTES}`);
  }
  return {
    resourceId: nonEmptyString(params.resourceId, "resourceId"),
    offset: nonNegativeInteger(params.offset, "offset"),
    maxBytes,
  };
}

function record(value: unknown, keys: readonly string[]): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("IPC params must be an object");
  }
  const params = value as Record<string, unknown>;
  const actualKeys = Object.keys(params).sort();
  const expectedKeys = [...keys].sort();
  if (
    actualKeys.length !== expectedKeys.length ||
    actualKeys.some((key, index) => key !== expectedKeys[index])
  ) {
    throw new Error(`IPC params must contain exactly: ${expectedKeys.join(", ")}`);
  }
  return params;
}

function nonEmptyString(value: unknown, field: string): string {
  const resolved = string(value, field);
  if (resolved.trim().length === 0) throw new Error(`${field} must not be empty`);
  return resolved;
}

function string(value: unknown, field: string): string {
  if (typeof value !== "string") throw new Error(`${field} must be a string`);
  return value;
}

function nonNegativeInteger(value: unknown, field: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new Error(`${field} must be a non-negative safe integer`);
  }
  return value as number;
}

function positiveInteger(value: unknown, field: string): number {
  const resolved = nonNegativeInteger(value, field);
  if (resolved === 0) throw new Error(`${field} must be positive`);
  return resolved;
}
