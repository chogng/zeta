import type { FsGetMetadataParams, FsReadDirectoryParams, FsReadFileParams, ResourceMetadataParams, ResourceReadParams, ResourceReleaseParams, SessionCommandParams, SessionCreateParams, SessionModelSetParams, SessionReadParams, SessionSubscribeParams, SessionThreadArchiveParams, SessionThreadCreateParams, SessionThreadForkParams, SessionUnsubscribeParams, ThreadReadParams, ThreadSubscribeParams, ThreadUnsubscribeParams, TurnInterruptParams, TurnInteractionResolveParams, TurnStartParams, TypstCompileParams, WorkspaceSearchCancelParams, WorkspaceSearchReadParams, WorkspaceSearchStartParams } from "../../../../../generated/app-server/types.js";
import { APP_SERVER_METHODS } from "../../../../../generated/app-server/types.js";
import type { GitCommitParams, GitPathsParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "./app-server-supervisor.js";
import { boolean, boundedPositiveInteger, nonEmptyString, nonNegativeInteger, positiveInteger, record, string, stringEnum } from "./app-server-ipc-validation.js";
import { terminalIpcRoutes } from "./terminal-ipc.js";
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
      channel: "zeta:app-server:slash-commands",
      validate: emptyParams,
      invoke: () => supervisor.slashCommands,
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
      channel: "zeta:turn:interaction:resolve",
      validate: turnInteractionResolveParams,
      invoke: (params) =>
        supervisor.request(
          APP_SERVER_METHODS["turn/interaction/resolve"],
          params,
        ),
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
    route({
      channel: "zeta:fs:get-metadata",
      validate: fsGetMetadataParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["fs/getMetadata"], params),
    }),
    route({
      channel: "zeta:fs:read-directory",
      validate: fsReadDirectoryParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["fs/readDirectory"], params),
    }),
    route({
      channel: "zeta:fs:read-file",
      validate: fsReadFileParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["fs/readFile"], params),
    }),
    route({
      channel: "zeta:git:status",
      validate: emptyParams,
      invoke: () =>
        supervisor.request(APP_SERVER_METHODS["git/status"], {}),
    }),
    route({
      channel: "zeta:git:history",
      validate: emptyParams,
      invoke: () =>
        supervisor.request(APP_SERVER_METHODS["git/history"], {}),
    }),
    route({
      channel: "zeta:git:stage",
      validate: gitPathsParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["git/stage"], params),
    }),
    route({
      channel: "zeta:git:unstage",
      validate: gitPathsParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["git/unstage"], params),
    }),
    route({
      channel: "zeta:git:discard-worktree",
      validate: gitPathsParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["git/discardWorktree"], params),
    }),
    route({
      channel: "zeta:git:commit",
      validate: gitCommitParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["git/commit"], params),
    }),
    route({
      channel: "zeta:git:fetch",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["git/fetch"], {}),
    }),
    route({
      channel: "zeta:git:pull",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["git/pull"], {}),
    }),
    route({
      channel: "zeta:git:push",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["git/push"], {}),
    }),
    route({
      channel: "zeta:workspace-search:start",
      validate: workspaceSearchStartParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["workspace/search/start"], params),
    }),
    route({
      channel: "zeta:workspace-search:read",
      validate: workspaceSearchReadParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["workspace/search/read"], params),
    }),
    route({
      channel: "zeta:workspace-search:cancel",
      validate: workspaceSearchCancelParams,
      invoke: (params) =>
        supervisor.request(APP_SERVER_METHODS["workspace/search/cancel"], params),
    }),
    ...terminalIpcRoutes(supervisor),
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

function fsGetMetadataParams(value: unknown): FsGetMetadataParams {
  const params = record(value, ["path"]);
  return { path: relativeWorkspacePath(params.path) };
}

function fsReadDirectoryParams(value: unknown): FsReadDirectoryParams {
  return fsGetMetadataParams(value);
}

function turnInteractionResolveParams(
  value: unknown,
): TurnInteractionResolveParams {
  const params = record(value, [
    "commandId",
    "sessionId",
    "threadId",
    "turnId",
    "requestId",
    "expectedSequence",
    "response",
  ]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    sessionId: nonEmptyString(params.sessionId, "sessionId"),
    threadId: nonEmptyString(params.threadId, "threadId"),
    turnId: nonEmptyString(params.turnId, "turnId"),
    requestId: nonEmptyString(params.requestId, "requestId"),
    expectedSequence: nonNegativeInteger(
      params.expectedSequence,
      "expectedSequence",
    ),
    response: agentResponse(params.response),
  };
}

function agentResponse(
  value: unknown,
): TurnInteractionResolveParams["response"] {
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
        response: {
          decision: stringEnum(
            decision.decision,
            "response.decision",
            ["approveOnce", "decline"] as const,
          ),
        },
      };
    }
    case "userInput": {
      const response = record(value, ["type", "response"]);
      const payload = record(response.response, ["answers"]);
      if (
        typeof payload.answers !== "object" ||
        payload.answers === null ||
        Array.isArray(payload.answers)
      ) {
        throw new Error("response.answers must be an object");
      }
      const answers: Record<string, { value: string }> = {};
      for (
        const [id, answer] of Object.entries(
          payload.answers as Record<string, unknown>,
        )
      ) {
        if (!id) throw new Error("response answer ID must not be empty");
        const item = record(answer, ["value"]);
        answers[id] = {
          value: string(item.value, `response.answers.${id}.value`),
        };
      }
      return { type, response: { answers } };
    }
    case "dynamicTool": {
      const response = record(value, ["type", "response"]);
      const payload = record(response.response, [
        "callId",
        "content",
        "success",
      ]);
      if (!Array.isArray(payload.content)) {
        throw new Error("response.content must be an array");
      }
      return {
        type,
        response: {
          callId: nonEmptyString(
            payload.callId,
            "response.callId",
          ),
          content: payload.content.map((entry, index) => {
            if (
              typeof entry !== "object" ||
              entry === null ||
              Array.isArray(entry)
            ) {
              throw new Error(`response.content[${index}] must be an object`);
            }
            const entryType = (entry as Record<string, unknown>).type;
            if (entryType === "text") {
              const text = record(entry, ["type", "text"]);
              return {
                type: "text" as const,
                text: string(
                  text.text,
                  `response.content[${index}].text`,
                ),
              };
            }
            if (entryType === "image") {
              const image = record(entry, ["type", "dataUrl"]);
              return {
                type: "image" as const,
                dataUrl: nonEmptyString(
                  image.dataUrl,
                  `response.content[${index}].dataUrl`,
                ),
              };
            }
            throw new Error(
              `response.content[${index}].type is unsupported`,
            );
          }),
          success: boolean(payload.success, "response.success"),
        },
      };
    }
    default:
      throw new Error("response.type is unsupported");
  }
}

function fsReadFileParams(value: unknown): FsReadFileParams {
  return fsGetMetadataParams(value);
}

function workspaceSearchStartParams(
  value: unknown,
): WorkspaceSearchStartParams {
  const params = record(value, [
    "query",
    "patternKind",
    "caseSensitivity",
    "includePatterns",
    "excludePatterns",
    "maxResults",
  ]);
  const query = nonEmptyString(params.query, "query");
  if (new TextEncoder().encode(query).byteLength > 16_384) {
    throw new Error("query must not exceed 16384 UTF-8 bytes");
  }
  return {
    query,
    patternKind: stringEnum(
      params.patternKind,
      "patternKind",
      ["literal", "regex"] as const,
    ),
    caseSensitivity: stringEnum(
      params.caseSensitivity,
      "caseSensitivity",
      ["smart", "sensitive", "insensitive"] as const,
    ),
    includePatterns: searchPatterns(
      params.includePatterns,
      "includePatterns",
    ),
    excludePatterns: searchPatterns(
      params.excludePatterns,
      "excludePatterns",
    ),
    maxResults: boundedPositiveInteger(
      params.maxResults,
      "maxResults",
      5_000,
    ),
  };
}

function workspaceSearchReadParams(
  value: unknown,
): WorkspaceSearchReadParams {
  const params = record(value, [
    "searchId",
    "afterMatch",
    "maxMatches",
  ]);
  return {
    searchId: nonEmptyString(params.searchId, "searchId"),
    afterMatch: nonNegativeInteger(params.afterMatch, "afterMatch"),
    maxMatches: boundedPositiveInteger(
      params.maxMatches,
      "maxMatches",
      200,
    ),
  };
}

function workspaceSearchCancelParams(
  value: unknown,
): WorkspaceSearchCancelParams {
  const params = record(value, ["searchId"]);
  return {
    searchId: nonEmptyString(params.searchId, "searchId"),
  };
}

function searchPatterns(value: unknown, field: string): string[] {
  if (!Array.isArray(value) || value.length > 64) {
    throw new Error(`${field} must be an array with at most 64 entries`);
  }
  return value.map((entry, index) => {
    const pattern = nonEmptyString(entry, `${field}[${index}]`);
    if (
      new TextEncoder().encode(pattern).byteLength > 1_024 ||
      pattern.includes("\0") ||
      pattern.startsWith("!") ||
      pattern.startsWith("/") ||
      /^[A-Za-z]:[\\/]/.test(pattern) ||
      pattern.replaceAll("\\", "/").split("/").includes("..")
    ) {
      throw new Error(
        `${field}[${index}] must be a workspace-relative glob`,
      );
    }
    return pattern;
  });
}

function relativeWorkspacePath(value: unknown): string {
  const path = string(value, "path");
  if (
    path.includes("\0") ||
    path.startsWith("/") ||
    path.startsWith("\\") ||
    /^[A-Za-z]:/.test(path) ||
    path.split(/[\\/]/).includes("..")
  ) {
    throw new Error("path must be relative to the workspace root");
  }
  return path;
}

function gitPathsParams(value: unknown): GitPathsParams {
  const params = record(value, ["paths"]);
  if (!Array.isArray(params.paths) || params.paths.length === 0 || params.paths.length > 5_000) {
    throw new Error("paths must contain between 1 and 5000 entries");
  }
  return {
    paths: params.paths.map((path, index) => {
      const resolved = relativeWorkspacePath(path);
      if (!resolved) throw new Error(`paths[${index}] must not be empty`);
      return resolved;
    }),
  };
}

function gitCommitParams(value: unknown): GitCommitParams {
  const params = record(value, ["message"]);
  const message = string(params.message, "message");
  if (!message.trim() || message.includes("\0") || new TextEncoder().encode(message).byteLength > 65_536) {
    throw new Error("message must be non-empty, NUL-free, and no larger than 65536 UTF-8 bytes");
  }
  return { message };
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
