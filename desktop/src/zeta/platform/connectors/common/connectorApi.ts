import type { ConnectorApiTokenConnectParams, ConnectorCommandResultDto, ConnectorDisconnectParams, ConnectorDisconnectResultDto, ConnectorListResult } from "../../../../../generated/app-server/types.js";

export interface ConnectorOAuthConnectParams {
  readonly commandId: string;
  readonly expectedGeneration: number;
  readonly connectorId: string;
  readonly connectionGeneration: number;
}

/** Transport-only Connector catalog and mutation operations. */
export interface IConnectorApi {
  list(): Promise<ConnectorListResult>;
  connectApiToken(params: ConnectorApiTokenConnectParams): Promise<ConnectorCommandResultDto>;
  connectOAuth(params: ConnectorOAuthConnectParams): Promise<ConnectorCommandResultDto>;
  disconnect(params: ConnectorDisconnectParams): Promise<ConnectorDisconnectResultDto>;
}
