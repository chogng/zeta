import { installBaseUiStyles } from "../../../base/browser/ui/index.js";
import type { IDisposable } from "../../../base/common/lifecycle.js";
import type { ZetaRendererApi } from "../../../platform/app-server/common/renderer-api.js";
import {
  UNKNOWN_EMPTY_WINDOW_WORKSPACE,
} from "../../../platform/workspace/common/workspace.js";
import { startWorkbench } from "../../../workbench/browser/workbench.js";
import {
  createBrowserTitlebarPart,
} from "../../../workbench/browser/parts/titlebar/titlebarPart.js";
import {
  createBrowserWorkbenchContextMenuService,
} from "../../../workbench/services/contextmenu/browser/contextMenuService.js";

/** Starts the web workbench after its browser host supplies the typed API bridge. */
export function startBrowserWorkbench(
  api: ZetaRendererApi,
  container: HTMLElement | null,
): IDisposable {
  installBaseUiStyles();
  return startWorkbench({
    api,
    container,
    workspace: UNKNOWN_EMPTY_WINDOW_WORKSPACE,
    createContextMenuService: createBrowserWorkbenchContextMenuService,
    createTitlebarPart: createBrowserTitlebarPart,
  });
}
