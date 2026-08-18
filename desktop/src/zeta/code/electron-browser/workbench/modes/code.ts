import "../../../browser/workbench/modes/code.contribution.js";
import { createElectronDebugAdapterCapability } from "../../../../platform/debug/electron-browser/electronDebugAdapterProcessService.js";
import { ZetaDesktopProduct } from "../../../../product/common/product.js";
import { codeSessionsProfile } from "../../../../sessions/browser/code/codeSessionsProfile.js";
import { registerSessionsTitlebarEntry } from "../../../../sessions/browser/common/sessionTitlebarEntry.js";
import { createSessionsWindowApi } from "../../../../sessions/electron-browser/sessionsWindowApi.js";
import { defaultWorkbenchProfile } from "../../../../workbench/browser/defaultWorkbenchProfile.js";
import { startElectronWorkbench } from "../../../../workbench/electron-browser/electronWorkbench.js";

registerSessionsTitlebarEntry(codeSessionsProfile.titlebarActionId, "Open Code Sessions", { kind: "window", sessionsWindowApi: createSessionsWindowApi() });
await startElectronWorkbench(ZetaDesktopProduct, defaultWorkbenchProfile, [createElectronDebugAdapterCapability]);
