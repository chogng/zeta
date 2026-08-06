import { installBaseUiStyles } from "../../base/browser/ui/styles.js";
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
import { type Workbench, startWorkbench } from "../browser/workbench.js";
import {
  createElectronTitlebarPartFactory,
} from "./parts/titlebar/titlebarPart.js";
import {
  createElectronWorkbenchContextMenuService,
} from "../services/contextmenu/electron-browser/contextMenuService.js";
import { loadUserThemes } from "./userThemes.js";
import type { WorkbenchSession } from "../browser/workbenchSession.js";

/** Starts one Electron renderer for the selected product edition. */
export async function startElectronWorkbench(
  product: ProductConfiguration,
  session: WorkbenchSession,
): Promise<void> {
  installBaseUiStyles();
  const disposableTracker = import.meta.env.DEV
    ? new DisposableTracker()
    : undefined;
  const tracking = disposableTracker
    ? installDisposableTracker(disposableTracker)
    : undefined;
  const api = createElectronRendererApi();
  const userThemes = await loadUserThemes(api.userThemes);
  const workbench = startWorkbench({
    product,
    session,
    api,
    container: document.querySelector<HTMLElement>("#app"),
    workspace: parseWorkspaceIdentifier(await api.workspace.getWorkspace()),
    configurationApi: api.configuration,
    keybindingsResourceApi: api.keybindings,
    nativeHostApi: api.nativeHost,
    userThemeService: userThemes,
    createContextMenuService: (options) =>
      createElectronWorkbenchContextMenuService(
        options,
        api.nativeContextMenu,
      ),
    createTitlebarPart: createElectronTitlebarPartFactory(
      api.nativeMenubar,
    ),
  });
  const workspaceSubscription = api.workspace.onDidChange((workspace) => {
    void applyWorkspaceChange(workbench, workspace);
  });
  window.addEventListener("pagehide", () => {
    try {
      workspaceSubscription.dispose();
      workbench.dispose();
      userThemes.dispose();
      disposableTracker?.assertNoLeaks();
    } finally {
      tracking?.[Symbol.dispose]();
    }
  }, { once: true });
}

async function applyWorkspaceChange(workbench: Workbench, workspace: unknown): Promise<void> {
  try {
    await workbench.updateWorkspace(parseWorkspaceIdentifier(workspace));
  } catch (error) {
    console.error("Failed to switch Workbench workspace", error);
    window.location.reload();
  }
}
