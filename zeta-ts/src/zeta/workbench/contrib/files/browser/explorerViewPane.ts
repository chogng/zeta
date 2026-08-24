import { IconLabel } from "../../../../base/browser/ui/iconlabel/iconlabel.js";
import { ScrollableElement } from "../../../../base/browser/ui/scrollbar/scrollableElement.js";
import { ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import { URI } from "../../../../base/common/uri.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import { FileKind, type IFileEntry, type IFileService } from "../../../../platform/files/common/files.js";
import type { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import type { IFileIconThemeService } from "../../../../platform/theme/browser/fileIconThemeService.js";
import type { IHoverService } from "../../../../platform/hover/common/hoverService.js";
import { WorkbenchAsyncDataTree, type ResourceOpenEvent } from "../../../../platform/list/browser/listService.js";
import type { IEditorService } from "../../../services/editor/common/editorService.js";
import { ViewPane, type IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import { h } from "../../../../base/browser/dom.js";

interface ExplorerNode {
	readonly resource: URI;
	readonly name: string;
	readonly kind: FileKind;
}

/** Workspace file tree backed by `IFileService` and the Workbench editor. */
export class ExplorerViewPane extends ViewPane {
	private readonly fileService: IFileService;
	private readonly workspaceContextService: IWorkspaceContextService;
	private readonly editorService: IEditorService;
	private readonly fileIconThemeService: IFileIconThemeService;
	private readonly hoverService: IHoverService;
	private readonly scrollable: ScrollableElement;
	private readonly tree: WorkbenchAsyncDataTree<ExplorerNode, ExplorerNode>;
	private readonly renderedLabels =
		this.own(new ResettableDisposableGroup());
	private root: ExplorerNode | undefined;
	private error: string | undefined;
	private workspaceGeneration = 0;

	constructor(
		container: HTMLElement,
		options: IViewPaneOptions,
		fileService: IFileService,
		workspaceContextService: IWorkspaceContextService,
		editorService: IEditorService,
		fileIconThemeService: IFileIconThemeService,
		hoverService: IHoverService,
		configurationService: IConfigurationService,
	) {
		super(container, options);
		this.fileService = fileService;
		this.workspaceContextService = workspaceContextService;
		this.editorService = editorService;
		this.fileIconThemeService = fileIconThemeService;
		this.hoverService = hoverService;
		this.element.classList.add("zeta-explorer-view-pane");
		this.headerElement.classList.add("zeta-explorer-title");
		this.contentElement.classList.add("zeta-explorer");
		this.scrollable = this.own(new ScrollableElement(this.contentElement, {
			ariaLabel: "Workspace files",
			direction: "vertical",
			vertical: "auto",
		}));
		this.tree = this.own(new WorkbenchAsyncDataTree<ExplorerNode, ExplorerNode>(this.scrollable.contentElement, {
			hasChildren: (node) => node.kind === FileKind.Directory,
			getChildren: async (node) => {
				const entries = await this.fileService.readDirectory(node.resource);
				return entries.map(explorerNode).sort(compareExplorerNodes);
			},
		}, {
			ariaLabel: "Workspace files",
			scrolling: "external",
			configurationService,
			indentGuides: "always",
			expandOnlyOnTwistieClick: false,
			identityProvider: { getId: (node) => node.resource.toString() },
			openOnSingleClick: true,
			onWillRender: () => this.renderedLabels.clear(),
			renderElement: (node) => this.renderTreeElement(node),
		}));
		this.own(this.tree.onDidError(({ error }) => {
			this.error = error instanceof Error ? error.message : "Unable to read workspace files.";
			this.render();
		}));
		this.own(this.tree.onDidOpen((event) => {
			if (event.element.kind === FileKind.File) void this.openFile(event);
		}));
		this.own(fileIconThemeService.onDidFileIconThemeChange(
			() => this.render(),
		));
		this.own(workspaceContextService.onDidChangeWorkspace(() => {
			void this.initialize();
		}));
		this.render();
		void this.initialize();
	}

	private async initialize(): Promise<void> {
		const generation = ++this.workspaceGeneration;
		this.root = undefined;
		this.error = undefined;
		void this.tree.setInput(undefined);
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
			if (this.isDisposed || generation !== this.workspaceGeneration) return;
			this.root = {
				resource: folder.uri,
				name: folder.name,
				kind: FileKind.Directory,
			};
			this.render();
			await this.tree.setInput(this.root);
		} catch (error) {
			if (this.isDisposed || generation !== this.workspaceGeneration) return;
			this.error = error instanceof Error
				? error.message
				: "Unable to load workspace files.";
			this.render();
		}
	}

	private async openFile(event: ResourceOpenEvent<ExplorerNode>): Promise<void> {
		const node = event.element;
		try {
			await this.editorService.openEditor({
				resource: node.resource,
				label: node.name,
			}, event.editorOptions, event.sideBySide ? "sideGroup" : "activeGroup");
		} catch (error) {
			if (this.isDisposed) return;
			this.error = error instanceof Error
				? error.message
				: `Unable to open ${node.name}.`;
			this.render();
		}
	}

	private render(): void {
		const document = this.element.ownerDocument;
		const surface = h(document, "div");
		surface.className = "zeta-explorer-scroll-content";
		if (!this.root) {
			const status = h(document, "div");
			status.className = "zeta-explorer-status";
			status.setAttribute("role", "status");
			status.textContent = this.error ?? "Loading files…";
			surface.append(status);
			this.scrollable.replaceChildren(surface);
			return;
		}
		if (this.error) {
			const error = h(document, "div");
			error.className = "zeta-explorer-status zeta-explorer-error";
			error.setAttribute("role", "alert");
			error.textContent = this.error;
			surface.append(error);
		}
		surface.append(this.tree.element);
		this.scrollable.replaceChildren(surface);
	}

	private renderTreeElement(node: ExplorerNode): HTMLElement {
		const document = this.element.ownerDocument;
		const content = h(document, "span");
		content.className = `zeta-explorer-row-content zeta-explorer-${node.kind}`;
		const label = this.renderedLabels.add(new IconLabel(content, {
			label: node.name,
			renderIcon: node.kind === FileKind.Directory
				? undefined
				: (container) => {
					this.fileIconThemeService.renderFileIcon(
						node.resource,
						container,
					);
				},
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
		return content;
	}
}

function explorerNode(entry: IFileEntry): ExplorerNode {
	return {
		resource: entry.resource,
		name: entry.name,
		kind: entry.kind,
	};
}

function compareExplorerNodes(left: ExplorerNode, right: ExplorerNode): number {
	const leftDirectory = left.kind === FileKind.Directory;
	const rightDirectory = right.kind === FileKind.Directory;
	if (leftDirectory !== rightDirectory) return leftDirectory ? -1 : 1;
	return left.name < right.name ? -1 : left.name > right.name ? 1 : 0;
}
