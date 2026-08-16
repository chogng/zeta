import { createServer } from "node:http";
import { randomUUID } from "node:crypto";
import { APP_SERVER_METHODS, type ConnectorApiTokenConnectParams, type ConnectorCommandResultDto, type ConnectorDeviceOAuthPollResult, type ConnectorDeviceOAuthStartResult, type ConnectorDisconnectParams, type ConnectorOAuthStartResult } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { ElectronClipboardService } from "../../clipboard/electron-main/electronClipboardService.js";
import type { IClipboardService } from "../../clipboard/common/clipboardService.js";
import { nonEmptyString, positiveInteger, record } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";
import { ElectronOpenerService } from "../../opener/electron-main/electronOpenerService.js";
import type { IOpenerService } from "../../opener/common/openerService.js";

export interface ConnectorHostServices {
  readonly openerService: IOpenerService;
  readonly clipboardService: IClipboardService;
}

export function connectorIpcRoutes(supervisor: AppServerSupervisor, hostServices: ConnectorHostServices = electronConnectorHostServices()): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({ channel: "zeta:connectors:list", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["connector/list"], {}) }),
    route({ channel: "zeta:connectors:connect-api-token", validate: connectParams, invoke: params => supervisor.request(APP_SERVER_METHODS["connector/connect/apiToken"], params) }),
    route({ channel: "zeta:connectors:connect-oauth", validate: oauthConnectParams, invoke: params => connectOAuth(supervisor, params, hostServices) }),
    route({ channel: "zeta:connectors:disconnect", validate: disconnectParams, invoke: params => supervisor.request(APP_SERVER_METHODS["connector/disconnect"], params) }),
    route({ channel: "zeta:connectors:oauth-refresh", validate: oauthRefreshParams, invoke: params => supervisor.request(APP_SERVER_METHODS["connector/oauth/refresh"], params) }),
    route({ channel: "zeta:connectors:oauth-revoke", validate: disconnectParams, invoke: params => supervisor.request(APP_SERVER_METHODS["connector/oauth/revoke"], params) }),
  ];
}

interface OAuthConnectParams {
  readonly commandId: string;
  readonly expectedGeneration: number;
  readonly connectorId: string;
  readonly connectionGeneration: number;
}

function oauthConnectParams(value: unknown): OAuthConnectParams {
  const params = record(value, ["commandId", "expectedGeneration", "connectorId", "connectionGeneration"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    expectedGeneration: positiveInteger(params.expectedGeneration, "expectedGeneration"),
    connectorId: nonEmptyString(params.connectorId, "connectorId"),
    connectionGeneration: positiveInteger(params.connectionGeneration, "connectionGeneration"),
  };
}

async function connectOAuth(supervisor: AppServerSupervisor, params: OAuthConnectParams, hostServices: ConnectorHostServices): Promise<ConnectorCommandResultDto> {
  const catalog = await supervisor.request(APP_SERVER_METHODS["connector/list"], {});
  const connector = catalog.connectors.find(candidate => candidate.id === params.connectorId);
  if (!connector) throw new Error("Connector is unavailable");
  if (connector.oauthMethods.includes("browser")) return connectBrowserOAuth(supervisor, params, hostServices.openerService);
  if (connector.oauthMethods.includes("device")) return connectDeviceOAuth(supervisor, params, hostServices);
  throw new Error("Connector OAuth is unavailable");
}

async function connectBrowserOAuth(supervisor: AppServerSupervisor, params: OAuthConnectParams, openerService: IOpenerService): Promise<ConnectorCommandResultDto> {
  const callbackPath = `/connector-oauth/${randomUUID()}`;
  const callback = await LoopbackOAuthCallback.listen(callbackPath);
  let flowId: string | undefined;
  let completed = false;
  try {
    const started = await supervisor.request(APP_SERVER_METHODS["connector/connect/oauth/start"], {
      ...params,
      redirectUri: callback.redirectUri,
    }) as ConnectorOAuthStartResult;
    flowId = started.flowId;
    await openerService.openExternal(started.authorizationUrl);
    const values = await callback.wait();
    const result = await supervisor.request(APP_SERVER_METHODS["connector/connect/oauth/complete"], {
      flowId: started.flowId,
      state: values.state,
      authorizationCode: values.code,
    }) as ConnectorCommandResultDto;
    completed = true;
    return result;
  } finally {
    callback.close();
    if (flowId && !completed) {
      await supervisor.request(APP_SERVER_METHODS["connector/connect/oauth/cancel"], { flowId }).catch(() => undefined);
    }
  }
}

async function connectDeviceOAuth(supervisor: AppServerSupervisor, params: OAuthConnectParams, hostServices: ConnectorHostServices): Promise<ConnectorCommandResultDto> {
  let flowId: string | undefined;
  let completed = false;
  try {
    const started = await supervisor.request(APP_SERVER_METHODS["connector/connect/oauth/device/start"], params) as ConnectorDeviceOAuthStartResult;
    flowId = started.flowId;
    await hostServices.openerService.openExternal(started.verificationUri);
    await hostServices.clipboardService.writeText(started.userCode);
    let waitSeconds = started.pollIntervalSeconds;
    for (;;) {
      await wait(Math.min(waitSeconds, 30) * 1_000);
      const result = await supervisor.request(APP_SERVER_METHODS["connector/connect/oauth/device/poll"], { flowId }) as ConnectorDeviceOAuthPollResult;
      if (result.status === "connected") {
        completed = true;
        return result.command;
      }
      waitSeconds = result.retryAfterSeconds;
    }
  } finally {
    if (flowId && !completed) {
      await supervisor.request(APP_SERVER_METHODS["connector/connect/oauth/device/cancel"], { flowId }).catch(() => undefined);
    }
  }
}

function electronConnectorHostServices(): ConnectorHostServices {
  return { openerService: new ElectronOpenerService(), clipboardService: new ElectronClipboardService() };
}

function wait(milliseconds: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, milliseconds));
}

class LoopbackOAuthCallback {
  private settled = false;
  private readonly completion: Promise<{ readonly state: string; readonly code: string }>;
  private resolve!: (value: { readonly state: string; readonly code: string }) => void;
  private reject!: (reason: Error) => void;
  private readonly timeout: NodeJS.Timeout;

  private constructor(private readonly server: ReturnType<typeof createServer>, readonly redirectUri: string, private readonly callbackPath: string) {
    this.completion = new Promise((resolve, reject) => {
      this.resolve = resolve;
      this.reject = reject;
    });
    this.timeout = setTimeout(() => this.finishError(new Error("Connector OAuth callback timed out")), 10 * 60 * 1000);
  }

  static async listen(callbackPath: string): Promise<LoopbackOAuthCallback> {
    const server = createServer();
    await new Promise<void>((resolve, reject) => {
      server.once("error", reject);
      server.listen(0, "127.0.0.1", () => {
        server.removeListener("error", reject);
        resolve();
      });
    });
    const address = server.address();
    if (!address || typeof address === "string") {
      server.close();
      throw new Error("Connector OAuth callback address is unavailable");
    }
    const callback = new LoopbackOAuthCallback(server, `http://127.0.0.1:${address.port}${callbackPath}`, callbackPath);
    server.on("request", (request, response) => callback.handle(request.url, response));
    server.on("error", () => callback.finishError(new Error("Connector OAuth callback host failed")));
    return callback;
  }

  wait(): Promise<{ readonly state: string; readonly code: string }> {
    return this.completion;
  }

  close(): void {
    clearTimeout(this.timeout);
    this.server.close();
  }

  private handle(rawUrl: string | undefined, response: import("node:http").ServerResponse): void {
    const url = new URL(rawUrl ?? "/", this.redirectUri);
    if (url.pathname !== this.callbackPath || this.settled) {
      response.writeHead(404).end();
      return;
    }
    const states = url.searchParams.getAll("state");
    const codes = url.searchParams.getAll("code");
    const errors = url.searchParams.getAll("error");
    response.setHeader("Content-Type", "text/plain; charset=utf-8");
    if (states.length !== 1 || codes.length !== 1 || errors.length !== 0 || !states[0] || !codes[0]) {
      response.writeHead(400).end("Authorization was not completed. You may close this window.");
      this.finishError(new Error("Connector OAuth provider did not return a valid callback"));
      return;
    }
    response.writeHead(200).end("Authorization complete. You may close this window and return to Zeta.");
    this.settled = true;
    clearTimeout(this.timeout);
    this.resolve({ state: states[0], code: codes[0] });
  }

  private finishError(error: Error): void {
    if (this.settled) return;
    this.settled = true;
    clearTimeout(this.timeout);
    this.reject(error);
    this.server.close();
  }
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

function oauthRefreshParams(value: unknown): { readonly connectorId: string } {
  const params = record(value, ["connectorId"]);
  return { connectorId: nonEmptyString(params.connectorId, "connectorId") };
}
