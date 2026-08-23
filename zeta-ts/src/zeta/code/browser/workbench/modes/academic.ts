import "./academic.contribution.js";
import { WorkbenchModeId } from "../../../../product/common/workbenchMode.js";
import { defaultWorkbenchProfile } from "../../../../workbench/browser/defaultWorkbenchProfile.js";
import { startBrowserWorkbench } from "../../../../workbench/browser/web.bootstrap.js";

startBrowserWorkbench(WorkbenchModeId.Academic, defaultWorkbenchProfile);
