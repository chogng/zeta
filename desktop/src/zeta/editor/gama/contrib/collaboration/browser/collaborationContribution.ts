import { ToolBar } from "../../../../../base/browser/ui/toolbar/toolbar.js";
import type { IContextMenuProvider } from "../../../../../base/browser/contextmenu.js";
import type { IAction } from "../../../../../base/common/actions.js";
import { lxiconsLibrary } from "../../../../../base/common/lxiconsLibrary.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { DocumentCollaborationTarget } from "../../../common/services/documentCollaborationService.js";

export type CollaborationToolbarState = "unavailable" | "inactive" | "connecting" | "connected" | "resyncRequired" | "error";

export interface CollaborationContributionOptions {
  readonly ownerDocument: Document;
  readonly onStart: (roomId: string | undefined, target: DocumentCollaborationTarget) => Promise<string>;
  readonly onStop: () => void;
}

/** Browser contribution that exposes Gama collaboration without owning document state or transport. */
export class CollaborationContribution extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly toolbar: ToolBar;
  private readonly status: HTMLSpanElement;
  private _state: CollaborationToolbarState = "unavailable";
  private roomId: string | undefined;
  private message: string | undefined;
  private target: DocumentCollaborationTarget | undefined;

  constructor(private readonly options: CollaborationContributionOptions) {
    super();
    const element = options.ownerDocument.createElement("div");
    element.className = "zeta-document-collaboration-toolbar";
    element.hidden = true;
    element.setAttribute("role", "group");
    element.setAttribute("aria-label", "Document collaboration");
    this.element = element;
    this.defer(() => element.remove());
    this.toolbar = this.own(new ToolBar({
      ownerDocument: options.ownerDocument,
      contextMenuProvider: emptyCollaborationContextMenuProvider,
      ariaLabel: "Document collaboration",
      highlightToggledItems: true,
    }));
    this.toolbar.element.classList.add("zeta-document-collaboration-actions");
    this.toolbar.element.addEventListener("mousedown", event => event.preventDefault());
    const status = options.ownerDocument.createElement("span");
    status.className = "zeta-document-collaboration-status";
    status.setAttribute("role", "status");
    this.status = status;
    element.append(this.toolbar.element, status);
    this.render();
  }

  setState(state: CollaborationToolbarState, options: { readonly roomId?: string; readonly message?: string; readonly target?: DocumentCollaborationTarget } = {}): void {
    this._state = state;
    this.roomId = options.roomId;
    this.message = options.message;
    if (options.target !== undefined) this.target = options.target;
    this.render();
  }

  private render(): void {
    const connected = this._state === "connected";
    const busy = this._state === "connecting";
    const enabled = this._state !== "unavailable" && !busy;
    this.element.dataset.state = this._state;
    this.toolbar.setActions([createAction(
      connected ? "stopCollaboration" : "startCollaboration",
      connected ? "Stop collaborating" : "Collaborate",
      connected ? "Leave this collaboration room" : "Create or join a collaboration room",
      enabled,
      connected,
      () => this.toggle(),
    )]);
    this.status.textContent = this.statusText();
  }

  private statusText(): string {
    switch (this._state) {
      case "unavailable": return "Collaboration unavailable";
      case "inactive": return "Share a room ID to collaborate";
      case "connecting": return "Connecting…";
      case "connected": return this.roomId ? `${this.target?.kind === "remote" ? "Remote room" : "Room"}: ${this.roomId}` : "Connected";
      case "resyncRequired": return this.roomId ? `Room ${this.roomId}: ${this.message ?? "rejoin required"}` : this.message ?? "Collaboration requires a resync";
      case "error": return this.roomId ? `Room ${this.roomId}: ${this.message ?? "collaboration failed"}` : this.message ?? "Collaboration failed";
    }
  }

  private toggle(): void {
    if (this._state === "connected") {
      this.options.onStop();
      return;
    }
    const target = this.requestTarget();
    if (!target) return;
    const entered = this.options.ownerDocument.defaultView?.prompt("Enter a collaboration room ID to join, or leave it blank to create one.", "");
    if (entered == null) return;
    this.setState("connecting");
    void this.options.onStart(entered.trim() || undefined, target).then(
      roomId => {
        if (this._state === "connecting") this.setState("connected", { roomId, target });
      },
      error => {
        if (this._state === "connecting") this.setState("error", { message: error instanceof Error ? error.message : "Collaboration could not be started" });
      },
    );
  }

  private requestTarget(): DocumentCollaborationTarget | undefined {
    const endpoint = this.options.ownerDocument.defaultView?.prompt("Enter a remote collaboration server URL, or leave it blank to use this renderer's App Server.", "");
    if (endpoint == null) return undefined;
    if (!endpoint.trim()) return { kind: "appServer" };
    const bearerToken = this.options.ownerDocument.defaultView?.prompt("Enter the remote collaboration server bearer token.", "");
    if (bearerToken == null) return undefined;
    return { kind: "remote", endpoint: endpoint.trim(), bearerToken: bearerToken.trim() };
  }
}

function createAction(id: string, label: string, tooltip: string, enabled: boolean, checked: boolean, run: () => void): IAction {
  return { id, label, tooltip, icon: lxiconsLibrary.agent, enabled, checked, run };
}

const emptyCollaborationContextMenuProvider: IContextMenuProvider = {
  showContextMenu(): never {
    throw new Error("Gama collaboration toolbar does not define secondary actions");
  },
};
