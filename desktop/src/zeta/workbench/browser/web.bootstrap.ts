import { URI } from "../../base/common/uri.js";
import type { ProductConfiguration } from "../../product/common/product.js";
import { connectViteDevRendererApi } from "../../platform/app-server/browser/webRendererApi.js";
import { startWebWorkbench } from "./web.factory.js";

declare const __ZETA_WEB_APP_SERVER__: boolean;

/** Starts a product Workbench after resolving its optional development host. */
export function startBrowserWorkbench(product: ProductConfiguration): void {
  void startBrowserWorkbenchAsync(product);
}

async function startBrowserWorkbenchAsync(product: ProductConfiguration): Promise<void> {
  if (globalThis.zetaWebWorkbenchHost !== undefined || !__ZETA_WEB_APP_SERVER__) {
    startWebWorkbench(product);
    return;
  }
  const hot = import.meta.hot;
  if (!hot) {
    console.error("Zeta Web App Server development mode requires the Vite hot channel");
    startWebWorkbench(product);
    return;
  }
  try {
    const connected = await connectViteDevRendererApi(hot);
    globalThis.zetaWebWorkbenchHost = {
      api: connected.api,
      workspace: Object.freeze({
        id: connected.metadata.workspaceId,
        uri: URI.file(connected.metadata.workspaceRoot),
      }),
    };
    window.addEventListener("pagehide", () => connected.dispose(), { once: true });
  } catch (error) {
    console.error("Failed to connect the Zeta Web development host", error);
  }
  startWebWorkbench(product);
}
