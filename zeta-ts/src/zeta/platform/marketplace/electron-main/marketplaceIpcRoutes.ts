import { APP_SERVER_METHODS, type MarketplaceAcquireCapabilityParams, type MarketplaceDownloadParams, type MarketplaceGetParams, type MarketplaceInstallParams, type MarketplaceOpenResourceParams, type MarketplaceReleaseCapabilityParams, type MarketplaceSearchParams, type MarketplaceUninstallParams, type MarketplaceUpdateParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonNegativeInteger, record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

export function marketplaceIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({ channel: "zeta:marketplace:search", validate: searchParams, invoke: params => supervisor.request(APP_SERVER_METHODS["marketplace/search"], params) }),
    route({ channel: "zeta:marketplace:get", validate: packageParams, invoke: params => supervisor.request(APP_SERVER_METHODS["marketplace/get"], params) }),
    route({ channel: "zeta:marketplace:download", validate: packageParams, invoke: params => supervisor.request(APP_SERVER_METHODS["marketplace/download"], params) }),
    route({ channel: "zeta:marketplace:install", validate: packageParams, invoke: params => supervisor.request(APP_SERVER_METHODS["marketplace/install"], params) }),
    route({ channel: "zeta:marketplace:update", validate: updateParams, invoke: params => supervisor.request(APP_SERVER_METHODS["marketplace/update"], params) }),
    route({ channel: "zeta:marketplace:uninstall", validate: uninstallParams, invoke: params => supervisor.request(APP_SERVER_METHODS["marketplace/uninstall"], params) }),
    route({ channel: "zeta:marketplace:list-installed", validate: emptyParams, invoke: params => supervisor.request(APP_SERVER_METHODS["marketplace/listInstalled"], params) }),
    route({ channel: "zeta:marketplace:acquire-capability", validate: acquireParams, invoke: params => supervisor.request(APP_SERVER_METHODS["marketplace/acquireCapability"], params) }),
    route({ channel: "zeta:marketplace:release-capability", validate: releaseParams, invoke: params => supervisor.request(APP_SERVER_METHODS["marketplace/releaseCapability"], params) }),
    route({ channel: "zeta:marketplace:open-resource", validate: openResourceParams, invoke: params => supervisor.request(APP_SERVER_METHODS["marketplace/openResource"], params) }),
  ];
}

function emptyParams(value: unknown): Record<string, never> {
  if (value === undefined) return {};
  return record(value, []) as Record<string, never>;
}

function searchParams(value: unknown): MarketplaceSearchParams {
  const params = record(value, ["query", "packageType", "limit"]);
  const limit = params.limit === null ? null : nonNegativeInteger(params.limit, "limit");
  if (limit !== null && (limit < 1 || limit > 200)) throw new Error("limit must be between 1 and 200");
  return { query: bounded(params.query, "query", 1024, true), packageType: nullableBounded(params.packageType, "packageType", 32), limit };
}

function packageParams(value: unknown): MarketplaceGetParams & MarketplaceDownloadParams & MarketplaceInstallParams {
  const params = record(value, ["packageId", "version"]);
  return { packageId: bounded(params.packageId, "packageId", 128), version: nullableBounded(params.version, "version", 128) };
}

function updateParams(value: unknown): MarketplaceUpdateParams {
  const params = record(value, ["installationId", "version"]);
  return { installationId: bounded(params.installationId, "installationId", 128), version: nullableBounded(params.version, "version", 128) };
}

function uninstallParams(value: unknown): MarketplaceUninstallParams {
  const params = record(value, ["installationId", "mode"]);
  const mode = string(params.mode, "mode");
  if (mode !== "ifUnused" && mode !== "whenUnused") throw new Error("mode is invalid");
  return { installationId: bounded(params.installationId, "installationId", 128), mode };
}

function acquireParams(value: unknown): MarketplaceAcquireCapabilityParams {
  const params = record(value, ["capability"]);
  const capability = record(params.capability, ["id"]);
  return { capability: { id: bounded(capability.id, "capability.id", 128) } };
}

function releaseParams(value: unknown): MarketplaceReleaseCapabilityParams {
  const params = record(value, ["leaseId"]);
  return { leaseId: bounded(params.leaseId, "leaseId", 128) };
}

function openResourceParams(value: unknown): MarketplaceOpenResourceParams {
  const params = record(value, ["leaseId", "resource"]);
  const resource = record(params.resource, ["id"]);
  return { leaseId: bounded(params.leaseId, "leaseId", 128), resource: { id: bounded(resource.id, "resource.id", 128) } };
}

function nullableBounded(value: unknown, field: string, maximum: number): string | null {
  return value === null ? null : bounded(value, field, maximum);
}

function bounded(value: unknown, field: string, maximum: number, empty = false): string {
  const candidate = string(value, field);
  if ((!empty && candidate.length === 0) || candidate.length > maximum) throw new Error(`${field} is invalid`);
  return candidate;
}

function route<P>(routeValue: IpcRoute<P, unknown>): IpcRoute<unknown, unknown> {
  return routeValue as IpcRoute<unknown, unknown>;
}
