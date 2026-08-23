import type { SkillListResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ISkillApi } from "../common/skillApi.js";
import { normalizeSkillCatalog } from "../common/skillApi.js";

export function createSkillApi(): ISkillApi {
	return { list: async (reload) => normalizeSkillCatalog(await invoke<SkillListResult>("zeta:skills:list", { reload })) };
}
