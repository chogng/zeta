import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/editor.code.all.js";
import "../../../workbench/contrib/codeEditor/browser/codeEditor.contribution.js";
import { codeWorkbenchSession } from "../../browser/workbench/codeWorkbenchSession.js";
import { codeSessionsProfile } from "../../../sessions/browser/code/codeSessionsProfile.js";
import { registerSessionsTitlebarEntry } from "../../../sessions/browser/common/sessionTitlebarEntry.js";
import { createSessionsWindowApi } from "../../../sessions/electron-browser/sessionsWindowApi.js";
import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { startElectronWorkbench } from "../../../workbench/electron-browser/electronWorkbench.js";

registerSessionsTitlebarEntry(codeSessionsProfile.titlebarActionId, "Open Code Sessions", { kind: "window", sessionsWindowApi: createSessionsWindowApi() });
await startElectronWorkbench(ZetaDesktopProduct, codeWorkbenchSession);
