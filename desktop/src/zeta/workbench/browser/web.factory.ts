import { addDisposableListener } from "../../base/browser/dom.js";
import { installBaseUiStyles } from "../../base/browser/ui/styles.js";
import {
  DisposableStore,
  type IDisposable,
} from "../../base/common/lifecycle.js";
import type {
  ProductConfiguration,
} from "../../product/common/product.js";
import {
  createDisconnectedRendererApi,
} from "../../platform/app-server/browser/rendererApi.js";
import {
  UNKNOWN_EMPTY_WINDOW_WORKSPACE,
} from "../../platform/workspace/common/workspace.js";
import {
  createBrowserWorkbenchContextMenuService,
} from "../services/contextmenu/browser/contextMenuService.js";
import {
  createBrowserTitlebarPart,
} from "./parts/titlebar/titlebarPart.js";
import type {
  IWebWorkbench,
  IWebWorkbenchConstructionOptions,
  IWebWorkbenchHost,
} from "./web.api.js";
import { startWorkbench } from "./workbench.js";

/** Creates a browser-hosted Workbench with the shared Web adapters. */
export function createWebWorkbench(
  product: ProductConfiguration,
  options: IWebWorkbenchConstructionOptions,
): IWebWorkbench {
  installBaseUiStyles();
  return startWorkbench({
    product,
    api: options.api,
    container: options.container,
    workspace: options.workspace ?? UNKNOWN_EMPTY_WINDOW_WORKSPACE,
    createContextMenuService: createBrowserWorkbenchContextMenuService,
    createTitlebarPart: createBrowserTitlebarPart,
  });
}

/**
 * Starts a product page from the optional global Web host and owns page
 * shutdown. A page without an embedder starts in an explicit disconnected
 * state so its UI remains inspectable without claiming backend availability.
 */
export function startWebWorkbench(
  product: ProductConfiguration,
): IDisposable {
  const host = readWebWorkbenchHost();
  const workbench = new DisposableStore();
  workbench.add(createWebWorkbench(product, {
    api: host?.api ?? createDisconnectedRendererApi(),
    workspace: host?.workspace,
    container: host?.container ??
      document.querySelector<HTMLElement>("#app"),
  }));
  workbench.add(addDisposableListener(window, "pagehide", () => {
    workbench.dispose();
  }, { once: true }));
  return workbench;
}

function readWebWorkbenchHost(): IWebWorkbenchHost | undefined {
  const host = globalThis.zetaWebWorkbenchHost;
  if (host === undefined) return undefined;
  if (
    typeof host !== "object" ||
    host === null ||
    typeof host.api !== "object" ||
    host.api === null
  ) {
    throw new TypeError(
      "globalThis.zetaWebWorkbenchHost must provide a renderer API",
    );
  }
  return host;
}
