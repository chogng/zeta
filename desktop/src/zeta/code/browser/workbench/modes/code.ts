import "./code.contribution.js";
import { createViteDevDebugAdapterCapability } from "../../../../platform/debug/browser/viteDevDebugAdapterProcessService.js";
import { ZetaDesktopProduct } from "../../../../product/common/product.js";
import { codeSessionsProfile } from "../../../../sessions/browser/code/codeSessionsProfile.js";
import { registerSessionsTitlebarEntry } from "../../../../sessions/browser/common/sessionTitlebarEntry.js";
import { defaultWorkbenchProfile } from "../../../../workbench/browser/defaultWorkbenchProfile.js";
import { startBrowserWorkbench } from "../../../../workbench/browser/web.bootstrap.js";

registerSessionsTitlebarEntry(codeSessionsProfile.titlebarActionId, "Open Code Sessions", { kind: "page", relativePath: "../sessions/sessions-code.html" });
startBrowserWorkbench(ZetaDesktopProduct, defaultWorkbenchProfile, [createViteDevDebugAdapterCapability]);
