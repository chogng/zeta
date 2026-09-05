import "../../../browser/workbench/modes/code.contribution.js";
import { createAppServerDebugAdapterCapability } from "../../../../platform/debug/browser/appServerDebugAdapterProcessService.js";
import { WorkbenchModeId } from "../../../../workbench/common/workbenchMode.js";
import { codeSessionsProfile } from "../../../../sessions/browser/code/codeSessionsProfile.js";
import { registerSessionsTitlebarEntry } from "../../../../sessions/browser/common/sessionTitlebarEntry.js";
import { createSessionsWindowApi } from "../../../../sessions/electron-browser/sessionsWindowApi.js";
import { startElectronWorkbench } from "../../../../workbench/electron-browser/electronWorkbench.js";

registerSessionsTitlebarEntry(codeSessionsProfile.titlebarActionId, "Open Code Sessions", { kind: "window", sessionsWindowApi: createSessionsWindowApi() });
await startElectronWorkbench(WorkbenchModeId.Code, [createAppServerDebugAdapterCapability]);
