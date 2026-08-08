import "../../../workbench/workbench.desktop.main.js";
import "../../../editor/alpha/editor.all.js";
import { codeWorkbenchSession } from "../../../sessions/browser/codeWorkbenchSession.js";
import { codeSessionsProfile } from "../../../sessions/browser/code/codeSessionsProfile.js";
import { registerSessionsTitlebarEntry } from "../../../sessions/browser/common/sessionTitlebarEntry.js";
import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { startElectronWorkbench } from "../../../workbench/electron-browser/electronWorkbench.js";

registerSessionsTitlebarEntry(codeSessionsProfile.titlebarActionId, "Open Code Sessions", "../sessions/sessions-code.html");
await startElectronWorkbench(ZetaDesktopProduct, codeWorkbenchSession);
