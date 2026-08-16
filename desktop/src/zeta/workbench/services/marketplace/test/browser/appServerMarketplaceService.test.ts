import assert from "node:assert/strict";
import test from "node:test";
import type { IMarketplaceApi } from "../../../../../platform/marketplace/common/marketplaceApi.js";
import { AppServerMarketplaceService } from "../../browser/appServerMarketplaceService.js";

test("Marketplace browse snapshots survive view recreation and invalidate after lifecycle changes", async () => {
  let searches = 0;
  let installedReads = 0;
  const summary = { id: "example/docs", version: "1.0.0", packageType: "mcp", displayName: "Docs", description: "Documentation search." };
  const packageReference = { id: summary.id, version: summary.version, digest: `sha256:${"a".repeat(64)}` };
  const details = { package: packageReference, packageType: summary.packageType, displayName: summary.displayName, description: summary.description, license: "MIT", source: "thirdParty" as const, upstream: null, capabilities: [] };
  const installed = { installationId: "installed-1", package: packageReference, state: "installed" as const, capabilities: [] };
  const api = {
    search: async () => {
      searches += 1;
      return { packages: [summary] };
    },
    get: async () => details,
    download: async () => ({ id: "artifact-1", package: packageReference }),
    install: async () => installed,
    update: async () => installed,
    uninstall: async () => {},
    listInstalled: async () => {
      installedReads += 1;
      return { packages: [] };
    },
    acquireCapability: async () => { throw new Error("unused"); },
    releaseCapability: async () => {},
    openResource: async () => { throw new Error("unused"); },
  } as IMarketplaceApi;
  const service = new AppServerMarketplaceService(api);

  const first = await service.browse("", undefined, 100);
  const reopened = await service.browse("", undefined, 100);
  assert.equal(reopened, first);
  assert.equal(service.cachedBrowse("", undefined, 100), first);
  assert.equal(searches, 1);
  assert.equal(installedReads, 1);

  await service.install(summary.id, summary.version);
  assert.equal(service.cachedBrowse("", undefined, 100), undefined);
  await service.browse("", undefined, 100);
  assert.equal(searches, 2);
  assert.equal(installedReads, 2);
});
