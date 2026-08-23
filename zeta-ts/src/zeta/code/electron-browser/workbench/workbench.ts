import "../../../workbench/workbench.desktop.main.js";
import { resolveWorkbenchModeIdFromUrl, WorkbenchModeId } from "../../../product/common/workbenchMode.js";

declare const __ZETA_WORKBENCH_MODE__: WorkbenchModeId;

const modeLoaders = {
	[WorkbenchModeId.Code]: () => import("./modes/code.js"),
	[WorkbenchModeId.Academic]: () => import("./modes/academic.js"),
} satisfies Record<WorkbenchModeId, () => Promise<unknown>>;

const modeId = resolveWorkbenchModeIdFromUrl(window.location.href, __ZETA_WORKBENCH_MODE__);
await modeLoaders[modeId]();
