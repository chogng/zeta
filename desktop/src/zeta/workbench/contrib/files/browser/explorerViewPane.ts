import { IconLabel } from "../../../../base/browser/ui/iconlabel/iconlabel.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import { ScrollableElement } from "../../../../base/browser/ui/scrollbar/scrollableElement.js";
import { Tree, type TreeTwistieState } from "../../../../base/browser/ui/tree/tree.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import { URI } from "../../../../base/common/uri.js";
import { FileKind, type IFileEntry, type IFileService } from "../../../../platform/files/common/files.js";
import type { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import type { IFileIconThemeService } from "../../../../platform/theme/browser/fileIconThemeService.js";
import type { IHoverService } from "../../../../platform/hover/common/hoverService.js";
import type { IEditorPart } from "../../../browser/parts/editor/editorPart.js";
import { ViewPane, type IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";

interface ExplorerNode {
  readonly resource: URI;
  readonly name: string;
  readonly kind: FileKind;
  expanded: boolean;
  loading: boolean;
  children: ExplorerNode[] | undefined;
}

/** Workspace file tree backed by `IFileService` and the Workbench editor. */
export class ExplorerViewPane extends ViewPane {
  private readonly fileService: IFileService;
  private readonly workspaceContextService: IWorkspaceContextService;
  private readonly editorPart: IEditorPart;
  private readonly fileIconThemeService: IFileIconThemeService;
  private readonly hoverService: IHoverService;
  private readonly scrollable: ScrollableElement;
  private readonly tree: Tree<ExplorerNode>;
  private readonly renderedLabels =
    this.own(new ResettableDisposableGroup());
  private root: ExplorerNode | undefined;
  private error: string | undefined;
  private disposed = false;
  private workspaceGeneration = 0;

  constructor(
    options: IViewPaneOptions,
    fileService: IFileService,
    workspaceContextService: IWorkspaceContextService,
    editorPart: IEditorPart,
    fileIconThemeService: IFileIconThemeService,
    hoverService: IHoverService,
  ) {
    super(options);
    this.fileService = fileService;
    this.workspaceContextService = workspaceContextService;
    this.editorPart = editorPart;
    this.fileIconThemeService = fileIconThemeService;
    this.hoverService = hoverService;
    this.element.classList.add("zeta-explorer-view-pane");
    this.headerElement.classList.add("zeta-explorer-title");
    this.contentElement.classList.add("zeta-explorer");
    this.scrollable = this.own(new ScrollableElement({
      ownerDocument: options.ownerDocument,
      ariaLabel: "Workspace files",
      direction: "vertical",
      vertical: "auto",
    }));
    this.contentElement.append(this.scrollable.element);
    this.tree = this.own(new Tree<ExplorerNode>({
      ownerDocument: options.ownerDocument,
      ariaLabel: "Workspace files",
      indentGuides: "always",
      getId: (node) => node.resource.toString(),
      getChildren: (node) => node.children,
      isCollapsible: (node) => node.kind === FileKind.Directory,
      isExpanded: (node) => node.expanded,
      renderElement: (node) => this.renderTreeElement(node),
      renderTwistie: (node, state, container) =>
        this.renderTreeTwistie(node, state, container),
    }));
    this.own(this.tree.onDidActivate(({ element }) => {
      this.activateNode(element);
    }));
    this.own(fileIconThemeService.onDidFileIconThemeChange(
      () => this.render(),
    ));
    this.own(workspaceContextService.onDidChangeWorkspace(() => {
      void this.initialize();
    }));
    this.defer(() => {
      this.disposed = true;
    });
    this.render();
    void this.initialize();
  }

  private async initialize(): Promise<void> {
    const generation = ++this.workspaceGeneration;
    this.root = undefined;
    this.error = undefined;
    this.render();
    const folder = this.workspaceContextService.getWorkspace().folders[0];
    if (!folder) {
      this.error = "Open a folder to browse files.";
      this.render();
      return;
    }
    try {
      this.setTitle(folder.name);
      const metadata = await this.fileService.stat(folder.uri);
      if (metadata.kind !== FileKind.Directory) {
        throw new Error("Workspace root is not a directory");
      }
      if (this.disposed || generation !== this.workspaceGeneration) return;
      this.root = {
        resource: folder.uri,
        name: folder.name,
        kind: FileKind.Directory,
        expanded: true,
        loading: false,
        children: undefined,
      };
      await this.loadChildren(this.root, generation);
    } catch (error) {
      if (this.disposed || generation !== this.workspaceGeneration) return;
      this.error = error instanceof Error
        ? error.message
        : "Unable to load workspace files.";
      this.render();
    }
  }

  private async loadChildren(
    node: ExplorerNode,
    generation: number = this.workspaceGeneration,
  ): Promise<void> {
    if (node.loading) return;
    node.loading = true;
    this.render();
    try {
      const entries = await this.fileService.readDirectory(node.resource);
      if (this.disposed || generation !== this.workspaceGeneration) return;
      node.children = entries.map(explorerNode).sort(compareExplorerNodes);
      node.expanded = true;
      this.error = undefined;
    } catch (error) {
      if (this.disposed || generation !== this.workspaceGeneration) return;
      this.error = error instanceof Error
        ? error.message
        : `Unable to read ${node.name}.`;
    } finally {
      node.loading = false;
      if (!this.disposed && generation === this.workspaceGeneration) {
        this.render();
      }
    }
  }

  private activateNode(node: ExplorerNode): void {
    if (node.loading) return;
    if (node.kind === FileKind.Directory) {
      if (node.children === undefined) {
        void this.loadChildren(node, this.workspaceGeneration);
        return;
      }
      node.expanded = !node.expanded;
      this.render();
    } else if (node.kind === FileKind.File) {
      void this.openFile(node);
    }
  }

  private async openFile(node: ExplorerNode): Promise<void> {
    try {
      await this.editorPart.openEditor({
        resource: node.resource,
        label: node.name,
      });
      if (!this.disposed) this.editorPart.focus();
    } catch (error) {
      if (this.disposed) return;
      this.error = error instanceof Error
        ? error.message
        : `Unable to open ${node.name}.`;
      this.render();
    }
  }

  private render(): void {
    const document = this.element.ownerDocument;
    this.renderedLabels.clear();
    this.tree.items = this.root?.children ?? [];
    const surface = document.createElement("div");
    surface.className = "zeta-explorer-scroll-content";
    if (!this.root) {
      const status = document.createElement("div");
      status.className = "zeta-explorer-status";
      status.textContent = this.error ?? "Loading files…";
      surface.append(status);
      this.scrollable.replaceChildren(surface);
      return;
    }
    surface.append(this.tree.element);
    if (this.error) {
      const error = document.createElement("div");
      error.className = "zeta-explorer-status zeta-explorer-error";
      error.textContent = this.error;
      surface.append(error);
    }
    this.scrollable.replaceChildren(surface);
  }

  private renderTreeElement(node: ExplorerNode): HTMLElement {
    const document = this.element.ownerDocument;
    const content = document.createElement("span");
    content.className = `zeta-explorer-row-content zeta-explorer-${node.kind}`;
    const label = this.renderedLabels.add(new IconLabel({
      label: node.name,
      renderIcon: node.kind === FileKind.Directory
        ? undefined
        : (container) => {
          this.fileIconThemeService.renderFileIcon(
            node.resource,
            container,
          );
        },
      ownerDocument: document,
      reserveIconSpace: node.kind !== FileKind.Directory,
    }));
    this.renderedLabels.add(this.hoverService.setupHover({
      target: label.element,
      content: () => label.labelElement.scrollWidth >
          label.labelElement.clientWidth
        ? node.name
        : undefined,
      groupId: "explorer.items",
    }));
    content.append(label.element);
    return content;
  }

  private renderTreeTwistie(node: ExplorerNode, state: TreeTwistieState, container: HTMLSpanElement): void {
    if (!state.collapsible) return;
    appendIcon(
      state.expanded
        ? lxiconsLibrary.dropdownIndicator
        : lxiconsLibrary.submenuIndicator,
      container,
    );
  }
}

function explorerNode(entry: IFileEntry): ExplorerNode {
  return {
    resource: entry.resource,
    name: entry.name,
    kind: entry.kind,
    expanded: false,
    loading: false,
    children: undefined,
  };
}

function compareExplorerNodes(left: ExplorerNode, right: ExplorerNode): number {
  const leftDirectory = left.kind === FileKind.Directory;
  const rightDirectory = right.kind === FileKind.Directory;
  if (leftDirectory !== rightDirectory) return leftDirectory ? -1 : 1;
  return left.name < right.name ? -1 : left.name > right.name ? 1 : 0;
}
