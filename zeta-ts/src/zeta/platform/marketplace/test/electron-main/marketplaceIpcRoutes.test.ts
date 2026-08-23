import { strict as assert } from "node:assert";
import test from "node:test";
import type { AppServerSupervisor } from "../../../../platform/app-server/electron-main/app-server-supervisor.js";
import { marketplaceIpcRoutes } from "../../../../platform/marketplace/electron-main/marketplaceIpcRoutes.js";

test("Marketplace list-installed IPC accepts the renderer's omitted params", async () => {
  const requests: unknown[] = [];
  const supervisor = {
    request: async (method: unknown, params: unknown) => {
      requests.push([method, params]);
      return { packages: [] };
    },
  } as unknown as AppServerSupervisor;
  const route = marketplaceIpcRoutes(supervisor).find(candidate => candidate.channel === "zeta:marketplace:list-installed");
  assert.ok(route);

  assert.deepEqual(await route.invoke(route.validate(undefined)), { packages: [] });
  assert.deepEqual(requests, [[{ method: "marketplace/listInstalled" }, {}]]);
  assert.throws(() => route.validate(null), /must be an object/);
  assert.throws(() => route.validate({ unexpected: true }), /exactly required keys/);
});
