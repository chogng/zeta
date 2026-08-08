import { installBaseUiStyles } from "../../base/browser/ui/styles.js";
import { DisposableStore, type IDisposable } from "../../base/common/lifecycle.js";
import type { ProductConfiguration } from "../../product/common/product.js";
import { createElectronRendererApi } from "../../platform/native/electron-browser/rendererApi.js";
import type { SessionsProfile } from "../common/sessionsProfile.js";
import { startSessionsWorkbench } from "../browser/sessionsWorkbench.js";

/** Starts an Electron Sessions page with native browser support when requested by Academic. */
export function startElectronSessions(product: ProductConfiguration, profile: SessionsProfile): IDisposable {
  installBaseUiStyles();
  const api = createElectronRendererApi();
  const sessions = new DisposableStore();
  sessions.add(startSessionsWorkbench({
    product,
    profile,
    api,
    browserViewApi: api.browserView,
    container: document.querySelector<HTMLElement>("#app"),
  }));
  window.addEventListener("pagehide", () => sessions.dispose(), { once: true });
  return sessions;
}
