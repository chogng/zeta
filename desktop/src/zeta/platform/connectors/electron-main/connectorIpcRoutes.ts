import { APP_SERVER_METHODS, type ConnectorApiTokenConnectParams, type ConnectorDisconnectParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonEmptyString, positiveInteger, record } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

export function connectorIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({ channel: "zeta:connectors:list", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["connector/list"], {}) }),
    route({ channel: "zeta:connectors:connect-api-token", validate: connectParams, invoke: params => supervisor.request(APP_SERVER_METHODS["connector/connect/apiToken"], params) }),
    route({ channel: "zeta:connectors:disconnect", validate: disconnectParams, invoke: params => supervisor.request(APP_SERVER_METHODS["connector/disconnect"], params) }),
  ];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
  return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}

function emptyParams(value: unknown): Record<string, never> {
  if (value === undefined) return {};
  return record(value, []) as Record<string, never>;
}

function connectParams(value: unknown): ConnectorApiTokenConnectParams {
  const params = record(value, ["commandId", "expectedGeneration", "connectorId", "connectionGeneration", "accountId", "accountDisplayName", "apiToken"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    expectedGeneration: positiveInteger(params.expectedGeneration, "expectedGeneration"),
    connectorId: nonEmptyString(params.connectorId, "connectorId"),
    connectionGeneration: positiveInteger(params.connectionGeneration, "connectionGeneration"),
    accountId: nonEmptyString(params.accountId, "accountId"),
    accountDisplayName: nonEmptyString(params.accountDisplayName, "accountDisplayName"),
    apiToken: nonEmptyString(params.apiToken, "apiToken"),
  };
}

function disconnectParams(value: unknown): ConnectorDisconnectParams {
  const params = record(value, ["commandId", "expectedGeneration", "connectorId"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    expectedGeneration: positiveInteger(params.expectedGeneration, "expectedGeneration"),
    connectorId: nonEmptyString(params.connectorId, "connectorId"),
  };
}
