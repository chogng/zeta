import { APP_SERVER_METHODS, type ToolSearchConfigureParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonEmptyString, nonNegativeInteger, record } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

export function toolSearchIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({ channel: "zeta:tool-search:config-read", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["config/read"], {}) }),
    route({ channel: "zeta:tool-search:configure", validate: configureParams, invoke: params => supervisor.request(APP_SERVER_METHODS["toolSearch/configure"], params) }),
  ];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
  return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}

function emptyParams(value: unknown): Record<string, never> {
  if (value === undefined) return {};
  return record(value, []) as Record<string, never>;
}

function configureParams(value: unknown): ToolSearchConfigureParams {
  const params = record(value, ["commandId", "expectedRevision", "mode"], ["embeddingModel"]);
  const mode = params.mode;
  if (mode !== "lexical" && mode !== "hybridEmbedding") throw new Error("mode must be lexical or hybridEmbedding");
  const embeddingModel = params.embeddingModel === null || params.embeddingModel === undefined
    ? null
    : modelRef(params.embeddingModel);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
    mode,
    embeddingModel,
  };
}

function modelRef(value: unknown): { provider: string; model: string } {
  const ref = record(value, ["provider", "model"]);
  return {
    provider: nonEmptyString(ref.provider, "embeddingModel.provider"),
    model: nonEmptyString(ref.model, "embeddingModel.model"),
  };
}
