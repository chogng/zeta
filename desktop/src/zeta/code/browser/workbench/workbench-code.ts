import "../../../workbench/workbench.web.main.js";
import "../../../editor/editor.code.all.js";
import "../../../workbench/contrib/codeEditor/browser/codeEditor.contribution.js";
import "../../../workbench/contrib/tasks/browser/tasks.contribution.js";
import "../../../workbench/contrib/testing/browser/testing.contribution.js";
import "../../../workbench/contrib/debug/browser/debug.contribution.js";
import "../../../workbench/contrib/extensionHost/browser/extensionHost.contribution.js";
import "../../../workbench/contrib/codeIntelligence/browser/codeIntelligence.contribution.js";
import { codeWorkbenchSession } from "./codeWorkbenchSession.js";
import { codeSessionsProfile } from "../../../sessions/browser/code/codeSessionsProfile.js";
import { registerSessionsTitlebarEntry } from "../../../sessions/browser/common/sessionTitlebarEntry.js";
import { ZetaDesktopProduct } from "../../../product/common/product.js";
import { startBrowserWorkbench } from "../../../workbench/browser/web.bootstrap.js";
import { createViteDevDebugAdapterCapability } from "../../../platform/debug/browser/viteDevDebugAdapterProcessService.js";

registerSessionsTitlebarEntry(codeSessionsProfile.titlebarActionId, "Open Code Sessions", { kind: "page", relativePath: "../sessions/sessions-code.html" });
startBrowserWorkbench(ZetaDesktopProduct, codeWorkbenchSession, [createViteDevDebugAdapterCapability]);
