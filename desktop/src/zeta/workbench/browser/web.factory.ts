import { addDisposableListener } from "../../base/browser/dom.js";
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
  startBrowserWorkbench,
} from "./browserWorkbench.js";
import type {
  IWebWorkbench,
  IWebWorkbenchConstructionOptions,
  IWebWorkbenchHost,
} from "./web.api.js";

/** Creates a browser-hosted Workbench for an explicit Web embedder. */
export function createWebWorkbench(
  product: ProductConfiguration,
  options: IWebWorkbenchConstructionOptions,
): IWebWorkbench {
  return startBrowserWorkbench(product, options);
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
