import { installBaseUiStyles } from "../../../base/browser/ui/index.js";
import type { ZetaRendererApi } from "../../../platform/app-server/common/renderer-api.js";
import { startWorkbench } from "../../../workbench/browser/workbench.js";

/** Starts the web workbench after its browser host supplies the typed API bridge. */
export function startBrowserWorkbench(api: ZetaRendererApi, container: Element | null): void {
  installBaseUiStyles();
  startWorkbench(api, container);
}
