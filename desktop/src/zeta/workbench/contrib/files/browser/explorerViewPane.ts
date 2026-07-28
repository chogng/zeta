import {
  addDisposableListener,
} from "../../../../base/browser/dom.js";
import { URI } from "../../../../base/common/uri.js";
import {
  FileKind,
  type IFileEntry,
  type IFileService,
} from "../../../../platform/files/common/files.js";
import type {
  IWorkspaceContextService,
} from "../../../../platform/workspace/common/workspace.js";
import {
  ViewPane,
  type IViewPaneOptions,
} from "../../../browser/parts/views/viewPane.js";

interface ExplorerNode {
  readonly resource: URI;
  readonly name: string;
  readonly kind: FileKind;
  expanded: boolean;
  loading: boolean;
  children: ExplorerNode[] | undefined;
}

/** Read-only workspace tree backed by `IFileService`. */
export class ExplorerViewPane extends ViewPane {
  readonly #fileService: IFileService;
  readonly #workspaceContextService: IWorkspaceContextService;
  readonly #nodes = new Map<string, ExplorerNode>();
  #root: ExplorerNode | undefined;
  #error: string | undefined;
  #disposed = false;

  constructor(
    options: IViewPaneOptions,
    fileService: IFileService,
    workspaceContextService: IWorkspaceContextService,
  ) {
    super(options);
    this.#fileService = fileService;
    this.#workspaceContextService = workspaceContextService;
    this.contentElement.classList.add("zeta-explorer");
    this.own(addDisposableListener(
      this.contentElement,
      "click",
      (event) => this.#onClick(event),
    ));
    this.defer(() => {
      this.#disposed = true;
      this.#nodes.clear();
    });
    void this.#initialize();
  }

  async #initialize(): Promise<void> {
    const folder = this.#workspaceContextService.getWorkspace().folders[0];
    if (!folder) {
      this.#error = "Open a folder to browse files.";
      this.#render();
      return;
    }
    try {
      const metadata = await this.#fileService.stat(folder.uri);
      if (metadata.kind !== FileKind.Directory || this.#disposed) {
        throw new Error("Workspace root is not a directory");
      }
      this.#root = {
        resource: folder.uri,
        name: folder.name,
        kind: FileKind.Directory,
        expanded: true,
        loading: false,
        children: undefined,
      };
      await this.#loadChildren(this.#root);
    } catch (error) {
      if (this.#disposed) return;
      this.#error = error instanceof Error
        ? error.message
        : "Unable to load workspace files.";
      this.#render();
    }
  }

  async #loadChildren(node: ExplorerNode): Promise<void> {
    if (node.loading) return;
    node.loading = true;
    this.#render();
    try {
      const entries = await this.#fileService.readDirectory(node.resource);
      if (this.#disposed) return;
      node.children = entries.map(explorerNode).sort(compareExplorerNodes);
      node.expanded = true;
      this.#error = undefined;
    } catch (error) {
      if (this.#disposed) return;
      this.#error = error instanceof Error
        ? error.message
        : `Unable to read ${node.name}.`;
    } finally {
      node.loading = false;
      if (!this.#disposed) this.#render();
    }
  }

  #onClick(event: Event): void {
    const target = event.target;
    const HTMLElementConstructor =
      this.element.ownerDocument.defaultView?.HTMLElement;
    if (
      !HTMLElementConstructor ||
      !(target instanceof HTMLElementConstructor)
    ) return;
    const button = target.closest<HTMLButtonElement>(
      "button[data-explorer-resource]",
    );
    if (!button || !this.contentElement.contains(button)) return;
    const node = this.#nodes.get(button.dataset.explorerResource ?? "");
    if (!node || node.kind !== FileKind.Directory || node.loading) return;
    if (node.children === undefined) {
      void this.#loadChildren(node);
      return;
    }
    node.expanded = !node.expanded;
    this.#render();
  }

  #render(): void {
    const document = this.element.ownerDocument;
    this.#nodes.clear();
    if (!this.#root) {
      const status = document.createElement("div");
      status.className = "zeta-explorer-status";
      status.textContent = this.#error ?? "Loading files…";
      this.contentElement.replaceChildren(status);
      return;
    }
    const tree = document.createElement("ul");
    tree.className = "zeta-explorer-tree";
    tree.setAttribute("role", "tree");
    tree.append(this.#renderNode(this.#root, document));
    const children: Node[] = [tree];
    if (this.#error) {
      const error = document.createElement("div");
      error.className = "zeta-explorer-status zeta-explorer-error";
      error.textContent = this.#error;
      children.push(error);
    }
    this.contentElement.replaceChildren(...children);
  }

  #renderNode(node: ExplorerNode, document: Document): HTMLLIElement {
    const item = document.createElement("li");
    item.setAttribute("role", "treeitem");
    if (node.kind === FileKind.Directory) {
      item.setAttribute("aria-expanded", String(node.expanded));
    }
    const button = document.createElement("button");
    button.type = "button";
    button.className = `zeta-explorer-row zeta-explorer-${node.kind}`;
    const key = node.resource.toString();
    button.dataset.explorerResource = key;
    this.#nodes.set(key, node);
    const twistie = node.kind === FileKind.Directory
      ? node.loading
        ? "…"
        : node.expanded
          ? "▾"
          : "▸"
      : "";
    button.textContent = `${twistie} ${node.name}`.trimStart();
    item.append(button);
    if (node.expanded && node.children) {
      const group = document.createElement("ul");
      group.setAttribute("role", "group");
      for (const child of node.children) {
        group.append(this.#renderNode(child, document));
      }
      item.append(group);
    }
    return item;
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
