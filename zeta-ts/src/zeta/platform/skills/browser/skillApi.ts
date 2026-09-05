import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { AppServerProtocolClient } from "../../app-server/browser/appServerProtocolClient.js";
import { appServerRequest } from "../../app-server/browser/appServerRequest.js";
import type { ISkillApi } from "../common/skillApi.js";
import { normalizeSkillCatalog } from "../common/skillApi.js";

export function createDisconnectedSkillApi(unavailable: UnavailableOperation): ISkillApi {
	return { list: () => unavailable("skills.list") };
}

export function createAppServerSkillApi(connection: AppServerProtocolClient): ISkillApi {
	return { list: async (reload) => normalizeSkillCatalog(await appServerRequest(connection, "skills/list", { reload })) };
}
