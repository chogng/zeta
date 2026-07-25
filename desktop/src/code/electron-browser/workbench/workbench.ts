import { installBaseUiStyles } from "../../../base/browser/ui/index.js";
import type { ZetaRendererApi } from "../../../platform/app-server/common/renderer-api.js";
import { startWorkbench } from "../../../workbench/browser/workbench.js";

declare global {
  interface Window {
    zeta: ZetaRendererApi;
  }
}

installBaseUiStyles();
startWorkbench(window.zeta, document.querySelector("#app"));
