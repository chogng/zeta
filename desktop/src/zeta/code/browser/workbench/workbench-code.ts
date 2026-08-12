import "../../../workbench/workbench.web.main.js";
import "../../../editor/editor.code.all.js";
import { codeWorkbenchSession } from "./codeWorkbenchSession.js";
import { codeSessionsProfile } from "../../../sessions/browser/code/codeSessionsProfile.js";
import { registerSessionsTitlebarEntry } from "../../../sessions/browser/common/sessionTitlebarEntry.js";
import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";

registerSessionsTitlebarEntry(codeSessionsProfile.titlebarActionId, "Open Code Sessions", { kind: "page", relativePath: "../sessions/sessions-code.html" });
startBrowserWorkbench(ZetaDesktopProduct, codeWorkbenchSession);
