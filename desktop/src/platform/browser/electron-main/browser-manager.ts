import { WebContentsView } from "electron/main";
import { randomUUID } from "node:crypto";

export type BrowserTargetId = string;
export type BrowserAction =
  | { type: "navigate"; targetId: BrowserTargetId; url: string }
  | { type: "goBack"; targetId: BrowserTargetId }
  | { type: "reload"; targetId: BrowserTargetId };

/** Electron Main's authoritative owner of isolated browser targets and their origin policy. */
export class BrowserManager {
  #targets = new Map<BrowserTargetId, WebContentsView>();

  createTarget(initialUrl: string): BrowserTargetId {
    this.assertAllowedOrigin(initialUrl);
    const view = new WebContentsView({ webPreferences: { contextIsolation: true, nodeIntegration: false, sandbox: true, partition: `zeta-browser-${randomUUID()}` } });
    const targetId = `browser_target_${randomUUID()}`;
    this.#targets.set(targetId, view);
    void view.webContents.loadURL(initialUrl);
    view.webContents.once("destroyed", () => this.#targets.delete(targetId));
    return targetId;
  }

  observe(targetId: BrowserTargetId): { targetId: BrowserTargetId; url: string; title: string } {
    const view = this.target(targetId);
    return { targetId, url: view.webContents.getURL(), title: view.webContents.getTitle() };
  }

  async perform(action: BrowserAction): Promise<void> {
    const view = this.target(action.targetId);
    switch (action.type) {
      case "navigate": this.assertAllowedOrigin(action.url); await view.webContents.loadURL(action.url); break;
      case "goBack": if (view.webContents.canGoBack()) view.webContents.goBack(); break;
      case "reload": view.webContents.reload(); break;
    }
  }

  close(targetId: BrowserTargetId): void {
    const view = this.target(targetId);
    this.#targets.delete(targetId);
    view.webContents.close();
  }

  private target(targetId: BrowserTargetId): WebContentsView {
    const target = this.#targets.get(targetId);
    if (!target || target.webContents.isDestroyed()) throw new Error("BrowserTargetUnavailable");
    return target;
  }

  private assertAllowedOrigin(value: string): void {
    const url = new URL(value);
    if (url.protocol !== "https:" && url.protocol !== "http:") throw new Error("BrowserOriginDenied");
  }
}
