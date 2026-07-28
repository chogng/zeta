import {
  WebContentsView,
  type BrowserWindow,
  type Event as ElectronEvent,
  type WebContents,
} from "electron/main";
import { randomUUID } from "node:crypto";
import type { EventEmitter } from "node:events";
import {
  DisposableOwner,
} from "../../../base/common/lifecycle.js";
import {
  type BrowserViewEvent,
  type BrowserViewTargetId,
  type IBrowserViewCreateRequest,
  type IBrowserViewLayoutRequest,
  type IBrowserViewNavigateRequest,
  type IBrowserViewState,
  type IBrowserViewVisibilityRequest,
  normalizeBrowserViewUrl,
} from "../common/browserView.js";
import type {
  IBrowserViewMainService,
} from "./browserViewIpc.js";

interface BrowserTarget {
  readonly id: BrowserViewTargetId;
  readonly view: WebContentsView;
  readonly disposables: DisposableStack;
  url: string;
  laidOut: boolean;
  visible: boolean;
}

export interface BrowserViewMainServiceOptions {
  readonly window: BrowserWindow;
  readonly emitEvent: (event: BrowserViewEvent) => void;
}

/**
 * Electron main's authoritative owner of isolated embedded browser targets.
 *
 * Every target belongs to one workbench window, starts hidden, and uses an
 * ephemeral session with no Node integration or granted web permissions.
 */
export class BrowserViewMainService extends DisposableOwner
  implements IBrowserViewMainService {
  readonly #window: BrowserWindow;
  readonly #emitEvent: (event: BrowserViewEvent) => void;
  readonly #targets = new Map<BrowserViewTargetId, BrowserTarget>();
  #disposing = false;

  constructor(options: BrowserViewMainServiceOptions) {
    super();
    this.#window = options.window;
    this.#emitEvent = options.emitEvent;
    this.defer(() => {
      this.#disposing = true;
      for (const target of [...this.#targets.values()]) {
        this.#releaseTarget(target, true);
      }
    });
  }

  createTarget(request: IBrowserViewCreateRequest): IBrowserViewState {
    const initialUrl = normalizeBrowserViewUrl(request.url);
    const targetId = `browser_target_${randomUUID()}`;
    const view = new WebContentsView({
      webPreferences: {
        contextIsolation: true,
        nodeIntegration: false,
        sandbox: true,
        webviewTag: false,
        partition: `zeta-browser-${randomUUID()}`,
      },
    });
    const target: BrowserTarget = {
      id: targetId,
      view,
      disposables: new DisposableStack(),
      url: initialUrl,
      laidOut: false,
      visible: false,
    };
    this.#targets.set(targetId, target);

    try {
      view.setBounds({ x: 0, y: 0, width: 1024, height: 768 });
      view.setVisible(false);
      this.#window.contentView.addChildView(view);
      this.#configureSecurity(target);
      this.#listen(target);
      void view.webContents.loadURL(initialUrl).catch(() => {
        // did-fail-load publishes the structured failure event.
      });
      return this.#state(target);
    } catch (error) {
      this.#releaseTarget(target, true);
      throw error;
    }
  }

  observe(targetId: string): IBrowserViewState {
    return this.#state(this.#target(targetId));
  }

  layout(request: IBrowserViewLayoutRequest): void {
    const target = this.#target(request.targetId);
    target.view.setBounds(request.bounds);
    target.laidOut = true;
  }

  setVisibility(request: IBrowserViewVisibilityRequest): void {
    const target = this.#target(request.targetId);
    if (request.visible && !target.laidOut) {
      throw new Error("BrowserTargetNotLaidOut");
    }
    if (target.visible === request.visible) return;
    target.visible = request.visible;
    target.view.setVisible(request.visible);
    this.#emitState(target);
  }

  async navigate(request: IBrowserViewNavigateRequest): Promise<void> {
    const target = this.#target(request.targetId);
    target.url = normalizeBrowserViewUrl(request.url);
    await target.view.webContents.loadURL(target.url);
  }

  goBack(targetId: string): void {
    const history = this.#target(targetId).view.webContents.navigationHistory;
    if (history.canGoBack()) history.goBack();
  }

  goForward(targetId: string): void {
    const history = this.#target(targetId).view.webContents.navigationHistory;
    if (history.canGoForward()) history.goForward();
  }

  reload(targetId: string): void {
    this.#target(targetId).view.webContents.reload();
  }

  stop(targetId: string): void {
    this.#target(targetId).view.webContents.stop();
  }

  close(targetId: string): void {
    this.#releaseTarget(this.#target(targetId), true);
  }

  #configureSecurity(target: BrowserTarget): void {
    const contents = target.view.webContents;
    const browserSession = contents.session;
    browserSession.setPermissionCheckHandler(() => false);
    browserSession.setPermissionRequestHandler(
      (_webContents, _permission, callback) => callback(false),
    );
    browserSession.setDevicePermissionHandler(() => false);

    const preventDownload = (event: ElectronEvent): void =>
      event.preventDefault();
    browserSession.on("will-download", preventDownload);
    target.disposables.defer(() =>
      browserSession.removeListener("will-download", preventDownload)
    );

    contents.setWindowOpenHandler(({ url }) => {
      try {
        this.#emit({
          type: "openRequested",
          targetId: target.id,
          url: normalizeBrowserViewUrl(url),
        });
      } catch {
        // Invalid and privileged URLs are denied without entering renderer IPC.
      }
      return { action: "deny" };
    });
  }

  #listen(target: BrowserTarget): void {
    const contents = target.view.webContents;
    this.#on(contents, target, "did-start-loading", () =>
      this.#emitState(target));
    this.#on(contents, target, "did-stop-loading", () =>
      this.#emitState(target));
    this.#on(contents, target, "did-navigate", (
      _event: ElectronEvent,
      url: string,
    ) => {
      target.url = normalizeBrowserViewUrl(url);
      this.#emitState(target);
    });
    this.#on(contents, target, "page-title-updated", () =>
      this.#emitState(target));
    this.#on(
      contents,
      target,
      "did-fail-load",
      (
        _event: ElectronEvent,
        errorCode: number,
        errorDescription: string,
        validatedURL: string,
        isMainFrame: boolean,
      ) => {
        if (!isMainFrame) return;
        this.#emit({
          type: "loadFailed",
          targetId: target.id,
          url: validatedURL,
          errorCode,
          errorDescription,
        });
        this.#emitState(target);
      },
    );
    this.#on(contents, target, "render-process-gone", (
      _event: ElectronEvent,
      details: Electron.RenderProcessGoneDetails,
    ) => {
      this.#emit({
        type: "renderProcessGone",
        targetId: target.id,
        reason: details.reason,
      });
    });
    this.#on(contents, target, "will-navigate", (
      event: ElectronEvent,
      url: string,
    ) =>
      this.#validateNavigation(event, url));
    this.#on(contents, target, "will-redirect", (
      event: ElectronEvent,
      url: string,
    ) =>
      this.#validateNavigation(event, url));
    this.#on(contents, target, "will-attach-webview", (
      event: ElectronEvent,
    ) =>
      event.preventDefault());
    this.#on(contents, target, "destroyed", () =>
      this.#releaseTarget(target, false));
  }

  #validateNavigation(event: ElectronEvent, url: string): void {
    try {
      normalizeBrowserViewUrl(url);
    } catch {
      event.preventDefault();
    }
  }

  #on(
    contents: WebContents,
    target: BrowserTarget,
    event: string,
    listener: (...args: any[]) => void,
  ): void {
    const emitter = contents as unknown as EventEmitter;
    emitter.on(event, listener);
    target.disposables.defer(() => emitter.removeListener(event, listener));
  }

  #state(target: BrowserTarget): IBrowserViewState {
    const contents = target.view.webContents;
    if (contents.isDestroyed()) {
      throw new Error("BrowserTargetUnavailable");
    }
    const history = contents.navigationHistory;
    return {
      targetId: target.id,
      url: contents.getURL() || target.url,
      title: contents.getTitle(),
      loading: contents.isLoading(),
      canGoBack: history.canGoBack(),
      canGoForward: history.canGoForward(),
      visible: target.visible,
    };
  }

  #target(targetId: string): BrowserTarget {
    const target = this.#targets.get(targetId);
    if (!target || target.view.webContents.isDestroyed()) {
      throw new Error("BrowserTargetUnavailable");
    }
    return target;
  }

  #emitState(target: BrowserTarget): void {
    if (!this.#targets.has(target.id)) return;
    this.#emit({ type: "stateChanged", state: this.#state(target) });
  }

  #emit(event: BrowserViewEvent): void {
    if (!this.#disposing) this.#emitEvent(event);
  }

  #releaseTarget(target: BrowserTarget, closeContents: boolean): void {
    if (!this.#targets.delete(target.id)) return;
    target.disposables.dispose();
    if (!this.#window.isDestroyed()) {
      this.#window.contentView.removeChildView(target.view);
    }
    if (closeContents && !target.view.webContents.isDestroyed()) {
      target.view.webContents.close();
    }
    this.#emit({ type: "closed", targetId: target.id });
  }
}
