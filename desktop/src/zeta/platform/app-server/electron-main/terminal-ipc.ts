import { APP_SERVER_METHODS, type TerminalCloseParams, type TerminalCreateParams, type TerminalReadParams, type TerminalResizeParams, type TerminalWriteParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "./app-server-supervisor.js";
import { boundedPositiveInteger, nonEmptyString, nonNegativeInteger, record, string } from "./app-server-ipc-validation.js";
import type { IpcRoute } from "./trusted-ipc-router.js";

/** Exact-shape IPC routes for the App Server terminal contract. */
export function terminalIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({
      channel: "zeta:terminal:profile-list",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["terminal/profile/list"], {}),
    }),
    route({
      channel: "zeta:terminal:create",
      validate: terminalCreateParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["terminal/create"], params),
    }),
    route({
      channel: "zeta:terminal:write",
      validate: terminalWriteParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["terminal/write"], params),
    }),
    route({
      channel: "zeta:terminal:resize",
      validate: terminalResizeParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["terminal/resize"], params),
    }),
    route({
      channel: "zeta:terminal:read",
      validate: terminalReadParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["terminal/read"], params),
    }),
    route({
      channel: "zeta:terminal:close",
      validate: terminalCloseParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["terminal/close"], params),
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

function terminalCreateParams(value: unknown): TerminalCreateParams {
  const params = record(value, ["rows", "cols", "profile"]);
  return {
    rows: boundedPositiveInteger(params.rows, "rows", 512),
    cols: boundedPositiveInteger(params.cols, "cols", 512),
    profile: terminalProfileSelection(params.profile),
  };
}

function terminalProfileSelection(value: unknown): TerminalCreateParams["profile"] {
  const profile = value as Record<string, unknown>;
  if (typeof profile !== "object" || profile === null || Array.isArray(profile)) {
    throw new Error("profile must be an object");
  }
  if (profile.type === "default") {
    record(profile, ["type"]);
    return { type: "default" };
  }
  if (profile.type === "profile") {
    const selected = record(profile, ["type", "profileId"]);
    return {
      type: "profile",
      profileId: nonEmptyString(selected.profileId, "profile.profileId"),
    };
  }
  throw new Error("profile.type must be default or profile");
}

function terminalWriteParams(value: unknown): TerminalWriteParams {
  const params = record(value, ["terminalId", "data"]);
  const data = string(params.data, "data");
  if (data.length === 0) throw new Error("data must not be empty");
  if (new TextEncoder().encode(data).byteLength > 65_536) {
    throw new Error("data must not exceed 65536 UTF-8 bytes");
  }
  return {
    terminalId: nonEmptyString(params.terminalId, "terminalId"),
    data,
  };
}

function terminalResizeParams(value: unknown): TerminalResizeParams {
  const params = record(value, ["terminalId", "rows", "cols"]);
  return {
    terminalId: nonEmptyString(params.terminalId, "terminalId"),
    rows: boundedPositiveInteger(params.rows, "rows", 512),
    cols: boundedPositiveInteger(params.cols, "cols", 512),
  };
}

function terminalReadParams(value: unknown): TerminalReadParams {
  const params = record(value, ["terminalId", "afterSequence", "afterCommandSequence", "maxChunks"]);
  return {
    terminalId: nonEmptyString(params.terminalId, "terminalId"),
    afterSequence: nonNegativeInteger(params.afterSequence, "afterSequence"),
    afterCommandSequence: nonNegativeInteger(params.afterCommandSequence, "afterCommandSequence"),
    maxChunks: boundedPositiveInteger(params.maxChunks, "maxChunks", 128),
  };
}

function terminalCloseParams(value: unknown): TerminalCloseParams {
  const params = record(value, ["terminalId"]);
  return {
    terminalId: nonEmptyString(params.terminalId, "terminalId"),
  };
}
