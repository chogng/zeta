import type { DiffComputeResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IDiffApi } from "../common/diffApi.js";

export function createDiffApi(): IDiffApi {
	return {
		compute: request => invoke<DiffComputeResult>("zeta:diff:compute", request),
	};
}
