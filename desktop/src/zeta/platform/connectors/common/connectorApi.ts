import type { ConnectorApiTokenConnectParams, ConnectorCommandResultDto, ConnectorDisconnectParams, ConnectorDisconnectResultDto, ConnectorListResult } from "../../../../../generated/app-server/types.js";

/** Transport-only Connector catalog and mutation operations. */
export interface IConnectorApi {
  list(): Promise<ConnectorListResult>;
  connectApiToken(params: ConnectorApiTokenConnectParams): Promise<ConnectorCommandResultDto>;
  disconnect(params: ConnectorDisconnectParams): Promise<ConnectorDisconnectResultDto>;
}
