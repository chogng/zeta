import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { ISkillApi } from "../common/skillApi.js";
import { normalizeSkillCatalog } from "../common/skillApi.js";

export function createDisconnectedSkillApi(unavailable: UnavailableOperation): ISkillApi {
  return { list: () => unavailable("skills.list") };
}

export function createViteDevSkillApi(connection: ViteDevAppServerConnection): ISkillApi {
  return { list: async (reload) => normalizeSkillCatalog(await viteDevRequest(connection, "skills/list", { reload })) };
}
