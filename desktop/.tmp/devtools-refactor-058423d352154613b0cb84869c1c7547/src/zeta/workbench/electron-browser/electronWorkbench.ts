import { installBaseUiStyles } from "../../base/browser/ui/index.js";
import {
  DisposableTracker,
  installDisposableTracker,
} from "../../base/common/disposableTracker.js";
import type {
  ProductConfiguration,
} from "../../product/common/product.js";
import {
  createElectronRendererApi,
} from "../../platform/native/electron-browser/rendererApi.js";
import {
  parseWorkspaceIdentifier,
} from "../../platform/workspace/common/workspace.js";
import { startWorkbench } from "../browser/workbench.js";
import {
  createElectronTitlebarPartFactory,
} from "./parts/titlebar/titlebarPart.js";
import {
  createElectronWorkbenchContextMenuService,
} from "../services/contextmenu/electron-browser/contextMenuService.js";

/** Starts one Electron renderer for the selected product edition. */
export async function startElectronWorkbench(
  product: ProductConfiguration,
): Promise<void> {
  installBaseUiStyles();
  const disposableTracker = import.meta.env.DEV
    ? new DisposableTracker()
    : undefined;
  const tracking = disposableTracker
    ? installDisposableTracker(disposableTracker)
    : undefined;
  const api = createElectronRendererApi();
  const workspace = parseWorkspaceIdentifier(
    await api.workspace.getWorkspace(),
  );
  const workbench = startWorkbench({
    product,
    api,
    container: document.querySelector<HTMLElement>("#app"),
    workspace,
    configurationApi: api.configuration,
    keybindingsResourceApi: api.keybindings,
    nativeHostApi: api.nativeHost,
    createContextMenuService: (options) =>
      createElectronWorkbenchContextMenuService(
        options,
        api.nativeContextMenu,
      ),
    createTitlebarPart: createElectronTitlebarPartFactory(
      api.nativeMenubar,
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
}
