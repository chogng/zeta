import { addDisposableListener } from "../../../base/browser/dom.js";
import { DisposableOwner, toDisposable } from "../../../base/common/lifecycle.js";
import type { BrowserViewEvent, BrowserViewTargetId, IBrowserViewApi, IBrowserViewState } from "../../../platform/browser/common/browserView.js";

/** Session-owned controller for one Electron-native research browser surface. */
export class SessionBrowserSurface extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly api: IBrowserViewApi | undefined;
  private readonly address: HTMLInputElement;
  private readonly back: HTMLButtonElement;
  private readonly forward: HTMLButtonElement;
  private readonly reload: HTMLButtonElement;
  private readonly viewport: HTMLDivElement;
  private readonly message: HTMLParagraphElement;
  private targetId: BrowserViewTargetId | undefined;
  private visible = false;
  private state: IBrowserViewState | undefined;
  private resizeObserver: ResizeObserver | undefined;

  constructor(ownerDocument: Document, api: IBrowserViewApi | undefined) {
    super();
    this.api = api;
    this.element = ownerDocument.createElement("section");
    this.element.className = "zeta-sessions-browser-surface";
    const toolbar = ownerDocument.createElement("form");
    toolbar.className = "zeta-sessions-browser-toolbar";
    this.back = toolbarButton(ownerDocument, "Back");
    this.forward = toolbarButton(ownerDocument, "Forward");
    this.reload = toolbarButton(ownerDocument, "Reload");
    this.address = ownerDocument.createElement("input");
    this.address.type = "url";
    this.address.value = "https://www.zotero.org";
    this.address.className = "zeta-sessions-browser-address";
    this.address.setAttribute("aria-label", "Research browser address");
    toolbar.append(this.back, this.forward, this.reload, this.address);
    this.viewport = ownerDocument.createElement("div");
    this.viewport.className = "zeta-sessions-browser-viewport";
    this.message = ownerDocument.createElement("p");
    this.message.className = "zeta-sessions-browser-message";
    this.viewport.append(this.message);
    this.element.append(toolbar, this.viewport);
    this.own(addDisposableListener(toolbar, "submit", (event) => {
      event.preventDefault();
      void this.navigate(this.address.value);
    }));
    this.own(addDisposableListener(this.back, "click", () => void this.api?.goBack(this.targetRequest())));
    this.own(addDisposableListener(this.forward, "click", () => void this.api?.goForward(this.targetRequest())));
    this.own(addDisposableListener(this.reload, "click", () => void this.api?.reload(this.targetRequest())));
    this.resizeObserver = typeof ResizeObserver === "undefined" ? undefined : new ResizeObserver(() => this.layout());
    this.resizeObserver?.observe(this.viewport);
    this.defer(() => {
      this.resizeObserver?.disconnect();
      if (this.targetId) void this.api?.close({ targetId: this.targetId });
    });
    this.render();
  }

  setVisible(visible: boolean): void {
    if (this.visible === visible) return;
    this.visible = visible;
    if (visible) {
      void this.show();
    } else if (this.targetId) {
      void this.api?.setVisibility({ targetId: this.targetId, visible: false });
    }
  }

  private async show(): Promise<void> {
    if (!this.api) {
      this.render("The research browser is available in the Electron Sessions window.");
      return;
    }
    try {
      if (!this.targetId) {
        const state = await this.api.create({ url: this.address.value });
        this.targetId = state.targetId;
        this.state = state;
        const subscription = this.api.onDidEvent((event) => this.acceptEvent(event));
        this.own(toDisposable(() => subscription.dispose()));
      }
      this.layout();
      await this.api.setVisibility({ targetId: this.targetId, visible: true });
      this.render();
    } catch (error) {
      this.render(error instanceof Error ? error.message : "Unable to open the research browser.");
    }
  }

  private async navigate(url: string): Promise<void> {
    if (!this.api || !this.targetId) {
      await this.show();
      return;
    }
    try {
      await this.api.navigate({ targetId: this.targetId, url });
    } catch (error) {
      this.render(error instanceof Error ? error.message : "Unable to open that address.");
    }
  }

  private layout(): void {
    if (!this.api || !this.targetId || !this.visible) return;
    const bounds = this.viewport.getBoundingClientRect();
    if (bounds.width <= 0 || bounds.height <= 0) return;
    void this.api.layout({
      targetId: this.targetId,
      bounds: {
        x: Math.round(bounds.x),
        y: Math.round(bounds.y),
        width: Math.round(bounds.width),
        height: Math.round(bounds.height),
      },
    });
  }

  private acceptEvent(event: BrowserViewEvent): void {
    if (event.type === "stateChanged" && event.state.targetId === this.targetId) {
      this.state = event.state;
      this.address.value = event.state.url;
      this.render();
      return;
    }
    if (event.type === "loadFailed" && event.targetId === this.targetId) {
      this.render(`Unable to load ${event.url}: ${event.errorDescription}`);
      return;
    }
    if (event.type === "closed" && event.targetId === this.targetId) {
      this.targetId = undefined;
      this.state = undefined;
      this.render("Research browser closed.");
    }
  }

  private targetRequest(): { readonly targetId: BrowserViewTargetId } {
    if (!this.targetId) throw new Error("Research browser is not open");
    return { targetId: this.targetId };
  }

  private render(error: string | undefined = undefined): void {
    this.back.disabled = !this.state?.canGoBack;
    this.forward.disabled = !this.state?.canGoForward;
    this.reload.disabled = !this.targetId;
    this.message.hidden = this.targetId !== undefined && error === undefined;
    this.message.textContent = error ?? (this.targetId ? "" : "Open a source to start browsing.");
  }
}

function toolbarButton(ownerDocument: Document, label: string): HTMLButtonElement {
  const button = ownerDocument.createElement("button");
  button.type = "button";
  button.className = "zeta-sessions-browser-button";
  button.textContent = label;
  return button;
}
