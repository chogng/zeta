import type { ConnectorCommandResultDto, ConnectorDisconnectResultDto, ConnectorListResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IConnectorApi } from "../common/connectorApi.js";

export function createConnectorApi(): IConnectorApi {
  return {
    list: () => invoke<ConnectorListResult>("zeta:connectors:list"),
    connectApiToken: params => invoke<ConnectorCommandResultDto>("zeta:connectors:connect-api-token", params),
    disconnect: params => invoke<ConnectorDisconnectResultDto>("zeta:connectors:disconnect", params),
  };
}
