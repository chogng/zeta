import { Button } from "../../../../../base/browser/ui/button/button.js";
import {
  ViewPane,
  type IViewPaneOptions,
} from "../../../../browser/parts/views/viewPane.js";
import type {
  IWorkspaceOpenService,
} from "../../../../services/workspaces/browser/workspaceOpenService.js";
import { h } from "../../../../../base/browser/dom.js";

/**
 * Files pane shown when the current Workbench has no workspace folders.
 *
 * Folder selection remains a host capability; this view delegates through
 * `IWorkspaceOpenService` and never accesses the native filesystem directly.
 */
export class EmptyView extends ViewPane {
  static readonly ID = "zeta.emptyExplorer";
  static readonly TITLE = "No Folder Opened";

  private readonly openButton: Button;
  private readonly statusElement: HTMLParagraphElement;
  private readonly workspaceOpenService: IWorkspaceOpenService;
  private disposed = false;

  constructor(
    options: IViewPaneOptions,
    workspaceOpenService: IWorkspaceOpenService,
  ) {
    super(options);
    this.workspaceOpenService = workspaceOpenService;
    this.contentElement.classList.add("zeta-empty-explorer");
    this.defer(() => {
      this.disposed = true;
    });

    const message = h(options.ownerDocument, "p");
    message.className = "zeta-empty-explorer-message";
    message.textContent = "Open a folder to explore its files.";
    this.openButton = this.own(new Button({
      label: "Open Folder",
      ownerDocument: options.ownerDocument,
      enabled: workspaceOpenService.canOpenFolder,
      title: workspaceOpenService.canOpenFolder
        ? "Open Folder"
        : "Opening folders is unavailable in this host",
      onClick: () => {
        void this.openFolder();
      },
    }));
    this.openButton.element.classList.add(
      "zeta-empty-explorer-open-folder",
    );
    this.statusElement = h(options.ownerDocument, "p");
    this.statusElement.className = "zeta-empty-explorer-status";
    this.statusElement.setAttribute("role", "status");
    this.statusElement.setAttribute("aria-live", "polite");
    this.contentElement.append(
      message,
      this.openButton.element,
      this.statusElement,
    );
  }

  private async openFolder(): Promise<void> {
    if (!this.workspaceOpenService.canOpenFolder) return;
    this.openButton.enabled = false;
    this.statusElement.textContent = "";
    try {
      await this.workspaceOpenService.openFolder();
    } catch (error) {
      if (this.disposed) return;
      this.statusElement.textContent = error instanceof Error
        ? error.message
        : "Unable to open a folder.";
    } finally {
      if (!this.disposed) {
        this.openButton.enabled =
          this.workspaceOpenService.canOpenFolder;
      }
    }
  }
}
