import { installBaseUiStyles } from "../../../base/browser/ui/index.js";
import {
  DisposableTracker,
  installDisposableTracker,
} from "../../../base/common/disposableTracker.js";
import type {
  ZetaElectronRendererApi,
} from "../../../platform/native/common/rendererApi.js";
import {
  createElectronWorkbenchContextMenuService,
} from "../../../workbench/services/contextmenu/electron-browser/contextMenuService.js";
import { startWorkbench } from "../../../workbench/browser/workbench.js";
import {
  createElectronTitlebarPartFactory,
} from "../../../workbench/electron-browser/parts/titlebar/titlebarPart.js";

declare global {
  interface Window {
    zeta: ZetaElectronRendererApi;
  }
}

installBaseUiStyles();
const disposableTracker = import.meta.env.DEV
  ? new DisposableTracker()
  : undefined;
const tracking = disposableTracker
  ? installDisposableTracker(disposableTracker)
  : undefined;
const workbench = startWorkbench({
  api: window.zeta,
  container: document.querySelector<HTMLElement>("#app"),
  configurationApi: window.zeta.configuration,
  keybindingsResourceApi: window.zeta.keybindings,
  createContextMenuService: (options) =>
    createElectronWorkbenchContextMenuService(
      options,
      window.zeta.nativeContextMenu,
    ),
  createTitlebarPart: createElectronTitlebarPartFactory(
    window.zeta.nativeMenubar,
  ),
});
window.addEventListener("pagehide", () => {
  try {
    workbench.dispose();
    disposableTracker?.assertNoLeaks();
  } finally {
    tracking?.[Symbol.dispose]();
  }
}, { once: true });
