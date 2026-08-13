import { APP_SERVER_METHODS, type PluginMarketplaceCommandParams, type PluginPackageCommandParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonEmptyString, nonNegativeInteger, record } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

export function pluginIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({ channel: "zeta:plugins:list", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["plugin/list"], {}) }),
    route({ channel: "zeta:plugins:marketplace-list", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["plugin/marketplace/list"], {}) }),
    route({ channel: "zeta:plugins:install", validate: marketplaceCommandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["plugin/install"], params) }),
    route({ channel: "zeta:plugins:update", validate: marketplaceCommandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["plugin/update"], params) }),
    route({ channel: "zeta:plugins:rollback", validate: commandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["plugin/rollback"], params) }),
    route({ channel: "zeta:plugins:enable", validate: commandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["plugin/enable"], params) }),
    route({ channel: "zeta:plugins:disable", validate: commandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["plugin/disable"], params) }),
    route({ channel: "zeta:plugins:grant", validate: commandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["plugin/grant"], params) }),
    route({ channel: "zeta:plugins:revoke-grant", validate: commandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["plugin/revokeGrant"], params) }),
    route({ channel: "zeta:plugins:uninstall", validate: commandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["plugin/uninstall"], params) }),
  ];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
  return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}

function emptyParams(value: unknown): Record<string, never> {
  if (value === undefined) return {};
  return record(value, []) as Record<string, never>;
}

function commandParams(value: unknown): PluginPackageCommandParams {
  const params = record(value, ["commandId", "expectedRevision", "id", "version", "digest"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
    id: nonEmptyString(params.id, "id"),
    version: nonEmptyString(params.version, "version"),
    digest: nonEmptyString(params.digest, "digest"),
  };
}

function marketplaceCommandParams(value: unknown): PluginMarketplaceCommandParams {
  const params = record(value, ["commandId", "expectedRevision", "marketplaceId", "id", "version", "digest"]);
  return {
    commandId: nonEmptyString(params.commandId, "commandId"),
    expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
    marketplaceId: nonEmptyString(params.marketplaceId, "marketplaceId"),
    id: nonEmptyString(params.id, "id"),
    version: nonEmptyString(params.version, "version"),
    digest: nonEmptyString(params.digest, "digest"),
  };
}
