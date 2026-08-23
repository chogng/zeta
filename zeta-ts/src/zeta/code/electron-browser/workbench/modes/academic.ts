import "../../../browser/workbench/modes/academic.contribution.js";
import { WorkbenchModeId } from "../../../../product/common/workbenchMode.js";
import { defaultWorkbenchProfile } from "../../../../workbench/browser/defaultWorkbenchProfile.js";
import { startElectronWorkbench } from "../../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(WorkbenchModeId.Academic, defaultWorkbenchProfile);
