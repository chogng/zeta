import { installBaseUiStyles } from "../../base/browser/ui/styles.js";
import { DisposableStore, type IDisposable } from "../../base/common/lifecycle.js";
import type { ProductConfiguration } from "../../product/common/product.js";
import { createElectronRendererApi } from "../../platform/native/electron-browser/rendererApi.js";
import type { SessionsProfile } from "../common/sessionsProfile.js";
import { startSessionsWorkbench } from "../browser/sessionsWorkbench.js";
import { createSessionsWindowApi } from "./sessionsWindowApi.js";

/** Starts the Code-specific Electron Sessions page. */
export function startElectronSessions(product: ProductConfiguration, profile: SessionsProfile): IDisposable {
  installBaseUiStyles();
  const api = createElectronRendererApi();
  const sessions = new DisposableStore();
  sessions.add(startSessionsWorkbench({
    product,
    profile,
    api,
    sessionsWindowApi: createSessionsWindowApi(),
    container: document.querySelector<HTMLElement>("#app"),
  }));
  window.addEventListener("pagehide", () => sessions.dispose(), { once: true });
  return sessions;
}
