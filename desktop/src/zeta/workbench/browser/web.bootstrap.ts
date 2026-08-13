import { URI } from "../../base/common/uri.js";
import type { ProductConfiguration } from "../../product/common/product.js";
import { connectViteDevRendererApi, type ViteDevRendererCapabilityContribution } from "../../platform/app-server/browser/webRendererApi.js";
import { startWebWorkbench } from "./web.factory.js";
import type { WorkbenchSession } from "./workbenchSession.js";

declare const __ZETA_WEB_APP_SERVER__: boolean;

/** Starts a product Workbench after resolving its optional development host. */
export function startBrowserWorkbench(product: ProductConfiguration, session: WorkbenchSession, rendererCapabilities: readonly ViteDevRendererCapabilityContribution[] = []): void {
  void startBrowserWorkbenchAsync(product, session, rendererCapabilities);
}

async function startBrowserWorkbenchAsync(product: ProductConfiguration, session: WorkbenchSession, rendererCapabilities: readonly ViteDevRendererCapabilityContribution[]): Promise<void> {
  if (globalThis.zetaWebWorkbenchHost !== undefined || !__ZETA_WEB_APP_SERVER__) {
    startWebWorkbench(product, session);
    return;
  }
  const hot = import.meta.hot;
  if (!hot) {
    console.error("Zeta Web App Server development mode requires the Vite hot channel");
    startWebWorkbench(product, session);
    return;
  }
  let disposeConnectedHost: (() => void) | undefined;
  try {
    const connected = await connectViteDevRendererApi(hot, {}, rendererCapabilities);
    globalThis.zetaWebWorkbenchHost = {
      api: connected.api,
      workspace: Object.freeze({
        id: connected.metadata.workspaceId,
        uri: URI.file(connected.metadata.workspaceRoot),
      }),
    };
    disposeConnectedHost = () => connected.dispose();
  } catch (error) {
    console.error("Failed to connect the Zeta Web development host", error);
  }
  try {
    startWebWorkbench(product, session);
  } catch (error) {
    disposeConnectedHost?.();
    throw error;
  }
  if (disposeConnectedHost) window.addEventListener("pagehide", disposeConnectedHost, { once: true });
}
