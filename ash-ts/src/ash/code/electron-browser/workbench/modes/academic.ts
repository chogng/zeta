import "../../../browser/workbench/modes/academic.contribution.js";
import { WorkbenchModeId } from "../../../../workbench/common/workbenchMode.js";
import { startElectronWorkbench } from "../../../../workbench/electron-browser/electronWorkbench.js";

await startElectronWorkbench(WorkbenchModeId.Academic);
