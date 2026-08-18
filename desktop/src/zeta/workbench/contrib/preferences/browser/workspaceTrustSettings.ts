import "./media/workspaceTrustSettings.css";
import { addDisposableListener, h } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import type { IWorkspaceTrustService, WorkspaceTrustEntry, WorkspaceTrustSnapshot } from "../../../../platform/workspaceTrust/common/workspaceTrustService.js";
import type { IWorkspaceOpenService } from "../../../services/workspaces/browser/workspaceOpenService.js";

/** Settings pane for reviewing and revoking durable User Workspace Trust decisions. */
export class WorkspaceTrustSettingsPane extends DisposableOwner {
  readonly element: HTMLDivElement;
  private active = true;

  constructor(ownerDocument: Document, private readonly service: IWorkspaceTrustService, private readonly workspaceOpenService: IWorkspaceOpenService, private readonly dialogService: IDialogService) {
    super();
    this.element = h(ownerDocument, "div");
    this.element.className = "zeta-workspace-trust-settings";
    this.defer(() => { this.active = false; });
    void this.load();
  }

  private async load(): Promise<void> {
    const document = this.element.ownerDocument;
    const loading = h(document, "p");
    loading.className = "zeta-settings-message";
    loading.textContent = "Loading Workspace Trust decisions…";
    this.element.replaceChildren(loading);
    try {
      const snapshot = await this.service.list();
      if (!this.active) return;
      this.render(snapshot);
    } catch (error) {
      if (!this.active) return;
      loading.textContent = error instanceof Error
        ? `Unable to load Workspace Trust decisions: ${error.message}`
        : "Unable to load Workspace Trust decisions.";
    }
  }

  private render(snapshot: WorkspaceTrustSnapshot): void {
    const document = this.element.ownerDocument;
    const intro = h(document, "p");
    intro.className = "zeta-workspace-trust-intro";
    intro.textContent = "Trusted folders can run terminals, Git mutations, tasks, and executable workspace configuration. Removing a decision returns the folder to Restricted Mode until you decide again.";

    const note = h(document, "p");
    note.className = "zeta-workspace-trust-note";
    note.textContent = "Trust checks use the folder's opaque canonical identity. The path shown here is display metadata only.";

    const toolbar = h(document, "div");
    toolbar.className = "zeta-workspace-trust-toolbar";
    const addFolder = h(document, "button");
    addFolder.className = "zeta-theme-action";
    addFolder.type = "button";
    addFolder.textContent = "Add Folder…";
    addFolder.disabled = !this.workspaceOpenService.canOpenFolder;
    addFolder.title = this.workspaceOpenService.canOpenFolder
      ? "Trust a folder"
      : "Folder picking is unavailable in this host";

    const list = h(document, "div");
    list.className = "zeta-workspace-trust-list";
    if (snapshot.entries.length === 0) {
      const empty = h(document, "p");
      empty.className = "zeta-workspace-trust-empty";
      empty.textContent = "No explicit Workspace Trust decisions have been saved.";
      list.append(empty);
    } else {
      for (const entry of [...snapshot.entries].sort(compareEntries)) list.append(this.renderEntry(entry, snapshot.revision, list));
    }
    this.own(addDisposableListener(addFolder, "click", () => {
      void this.addFolder(addFolder, list);
    }));
    toolbar.append(addFolder);
    this.element.replaceChildren(intro, note, toolbar, list);
  }

  private async addFolder(button: HTMLButtonElement, list: HTMLElement): Promise<void> {
    button.disabled = true;
    try {
      const root = await this.workspaceOpenService.pickFolder();
      if (!root || !this.active) return;
      const snapshot = await this.service.list();
      if (!this.active) return;
      await this.service.set(root, "trusted", snapshot.revision);
      if (this.active) await this.load();
    } catch (error) {
      if (!this.active) return;
      const failure = h(this.element.ownerDocument, "p");
      failure.className = "zeta-workspace-trust-error";
      failure.textContent = error instanceof Error ? `Unable to add trusted folder: ${error.message}` : "Unable to add trusted folder.";
      list.append(failure);
    } finally {
      if (this.active && this.element.contains(button)) button.disabled = false;
    }
  }

  private renderEntry(entry: WorkspaceTrustEntry, revision: number, list: HTMLElement): HTMLElement {
    const document = this.element.ownerDocument;
    const item = h(document, "article");
    item.className = "zeta-workspace-trust-entry";
    const copy = h(document, "div");
    copy.className = "zeta-workspace-trust-entry-copy";
    const path = h(document, "h4");
    path.textContent = entry.root ?? "Unknown folder (legacy trust record)";
    path.title = entry.root ?? entry.workspace;
    const status = h(document, "p");
    status.className = `zeta-workspace-trust-status ${entry.setting}`;
    status.textContent = entry.setting === "trusted" ? "Trusted" : "Restricted";
    const identity = h(document, "p");
    identity.className = "zeta-workspace-trust-identity";
    identity.textContent = entry.workspace;
    copy.append(path, status, identity);

    const actions = h(document, "div");
    actions.className = "zeta-workspace-trust-entry-actions";
    const forget = h(document, "button");
    forget.className = "zeta-theme-action";
    forget.type = "button";
    forget.textContent = entry.setting === "trusted" ? "Revoke Trust" : "Forget Decision";
    this.own(addDisposableListener(forget, "click", () => {
      void this.forgetEntry(entry, revision, item, forget, list);
    }));
    actions.append(forget);
    item.append(copy, actions);
    return item;
  }

  private async forgetEntry(entry: WorkspaceTrustEntry, revision: number, item: HTMLElement, button: HTMLButtonElement, list: HTMLElement): Promise<void> {
    const confirmed = await this.dialogService.confirm({
      title: entry.setting === "trusted" ? "Revoke Workspace Trust?" : "Forget Workspace Trust decision?",
      message: entry.root ?? entry.workspace,
      detail: "The folder will use Restricted Mode until you explicitly trust it again.",
      primaryButton: entry.setting === "trusted" ? "Revoke Trust" : "Forget Decision",
      cancelButton: "Cancel",
    });
    if (!confirmed) return;
    item.setAttribute("aria-busy", "true");
    button.disabled = true;
    try {
      await this.service.forget(entry.workspace, revision);
      await this.load();
    } catch (error) {
      button.disabled = false;
      item.removeAttribute("aria-busy");
      const failure = h(this.element.ownerDocument, "p");
      failure.className = "zeta-workspace-trust-error";
      failure.textContent = error instanceof Error ? `Unable to revoke trust: ${error.message}` : "Unable to revoke trust.";
      list.append(failure);
    }
  }
}

function compareEntries(left: WorkspaceTrustEntry, right: WorkspaceTrustEntry): number {
  return (left.root ?? left.workspace).localeCompare(right.root ?? right.workspace);
}
