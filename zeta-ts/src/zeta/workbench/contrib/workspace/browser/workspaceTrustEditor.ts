import "./media/workspaceTrustEditor.css";
import { h } from "../../../../base/browser/dom.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import type { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import type { IWorkspaceTrustService, WorkspaceTrustEntry, WorkspaceTrustSnapshot, WorkspaceTrustState } from "../../../../platform/workspaceTrust/common/workspaceTrustService.js";
import type { IWorkspaceOpenService } from "../../../services/workspaces/browser/workspaceOpenService.js";
import { setSettingsItemIdentity } from "../../preferences/browser/settingsItem.js";

interface CurrentWorkspaceTrust {
	readonly label: string;
	readonly root: string;
	readonly state: WorkspaceTrustState;
}

/** Editor surface owned by the Workspace contrib for current state and the durable trust allowlist. */
export class WorkspaceTrustEditor extends DisposableOwner {
	readonly element: HTMLDivElement;
	private readonly rendered = this.own(new ResettableDisposableGroup());
	private active = true;

	constructor(container: HTMLElement, private readonly service: IWorkspaceTrustService, private readonly workspaceOpenService: IWorkspaceOpenService, private readonly dialogService: IDialogService, private readonly workspaceContextService?: IWorkspaceContextService) {
		super();
		this.element = h(container.ownerDocument, "div");
		this.element.className = "zeta-workspace-trust-editor";
		container.append(this.element);
		this.defer(() => { this.active = false; });
		if (workspaceContextService) this.own(workspaceContextService.onDidChangeWorkspace(() => { void this.load(); }));
		void this.load();
	}

	private async load(): Promise<void> {
		this.rendered.clear();
		const document = this.element.ownerDocument;
		const loading = h(document, "p");
		loading.className = "zeta-settings-message";
		loading.textContent = "Loading Trusted Folders & Workspaces…";
		this.element.replaceChildren(loading);
		try {
			const snapshot = await this.service.list();
			const current = this.workspaceContextService ? await this.readCurrentWorkspaceTrust() : undefined;
			if (!this.active) return;
			this.render(snapshot, current);
		} catch (error) {
			if (!this.active) return;
			loading.textContent = error instanceof Error
				? `Unable to load Trusted Folders & Workspaces: ${error.message}`
				: "Unable to load Trusted Folders & Workspaces.";
		}
	}

	private render(snapshot: WorkspaceTrustSnapshot, current: CurrentWorkspaceTrust | undefined): void {
		const document = this.element.ownerDocument;
		const intro = h(document, "p");
		intro.className = "zeta-workspace-trust-intro";
		intro.textContent = "Trusted folders can run terminals, Git mutations, tasks, and executable workspace configuration. Removing a folder returns it to Restricted Mode until you decide again.";

		const note = h(document, "p");
		note.className = "zeta-workspace-trust-note";
		note.textContent = "Trust checks use the folder's opaque canonical identity. The path shown here is display metadata only.";

		const currentPanel = this.workspaceContextService ? this.renderCurrentWorkspace(snapshot, current) : undefined;

		const toolbar = h(document, "div");
		toolbar.className = "zeta-workspace-trust-toolbar";
		const addFolder = this.rendered.add(new Button(toolbar, {
			label: "Add Folder…",
			presentation: "primary",
			enabled: this.workspaceOpenService.canOpenFolder,
			title: this.workspaceOpenService.canOpenFolder
				? "Trust a folder"
				: "Folder picking is unavailable in this host",
			onClick: () => void this.addFolder(addFolder, list),
		}));

		const heading = h(document, "h4");
		heading.className = "zeta-workspace-trust-list-heading";
		heading.textContent = "Trusted Folders & Workspaces";
		const summary = h(document, "p");
		summary.className = "zeta-workspace-trust-list-description";
		summary.textContent = "You trust the following folders, their subfolders, and workspace files.";

		const list = h(document, "div");
		list.className = "zeta-workspace-trust-list";
		list.dataset.workspaceTrustList = "trusted";
		if (snapshot.entries.length === 0) {
			const empty = h(document, "p");
			empty.className = "zeta-workspace-trust-empty";
			empty.textContent = "You haven't trusted any folders or workspace files yet. Use Add Folder… to trust a folder.";
			list.append(empty);
		} else {
			for (const entry of [...snapshot.entries].sort(compareEntries)) list.append(this.renderEntry(entry, snapshot.revision, list));
		}
		this.element.replaceChildren(...(currentPanel ? [intro, note, currentPanel, toolbar, heading, summary, list] : [intro, note, toolbar, heading, summary, list]));
	}

	private async readCurrentWorkspaceTrust(): Promise<CurrentWorkspaceTrust | undefined> {
		const current = currentWorkspaceRoot(this.workspaceContextService?.getWorkspace());
		if (!current) return undefined;
		return { ...current, state: await this.service.read(current.root) };
	}

	private renderCurrentWorkspace(snapshot: WorkspaceTrustSnapshot, current: CurrentWorkspaceTrust | undefined): HTMLElement {
		const document = this.element.ownerDocument;
		const panel = h(document, "section");
		panel.className = "zeta-workspace-trust-current";
		setSettingsItemIdentity(panel, "workspaceTrust.current", "resource");
		const heading = h(document, "h4");
		heading.className = "zeta-workspace-trust-current-heading";
		heading.textContent = "Current Workspace";
		panel.append(heading);
		if (!current) {
			const status = h(document, "p");
			status.className = "zeta-workspace-trust-current-detail";
			status.textContent = "No single local folder is open, so there is no active Workspace Trust state to manage.";
			panel.append(status);
			return panel;
		}

		const path = h(document, "p");
		path.className = "zeta-workspace-trust-current-path";
		path.textContent = current.root;
		path.title = current.root;
		const status = h(document, "p");
		status.className = `zeta-workspace-trust-status ${current.state}`;
		status.textContent = current.state === "trusted" ? "Trusted" : "Restricted";
		const detail = h(document, "p");
		detail.className = "zeta-workspace-trust-current-detail";
		detail.textContent = current.state === "trusted"
			? "Workspace capabilities are enabled for this folder."
			: "Restricted Mode keeps terminals, Git mutations, tasks, executable workspace configuration, and executable language services disabled.";
		const actions = h(document, "div");
		actions.className = "zeta-workspace-trust-current-actions";
		const action = this.rendered.add(new Button(actions, {
			label: current.state === "trusted" ? "Revoke Trust" : `Trust This ${current.label}`,
			presentation: current.state === "trusted" ? "danger" : "primary",
			onClick: () => void this.updateCurrentTrust(current, snapshot.revision, action, panel),
		}));
		panel.append(path, status, detail, actions);
		return panel;
	}

	private async updateCurrentTrust(current: CurrentWorkspaceTrust, revision: number, button: Button, feedbackContainer: HTMLElement): Promise<void> {
		button.enabled = false;
		try {
			await this.service.set(current.root, current.state === "trusted" ? "restricted" : "trusted", revision);
			if (this.active) await this.load();
		} catch (error) {
			if (!this.active) return;
			const failure = h(this.element.ownerDocument, "p");
			failure.className = "zeta-workspace-trust-error";
			failure.textContent = error instanceof Error ? `Unable to update Workspace Trust: ${error.message}` : "Unable to update Workspace Trust.";
			feedbackContainer.append(failure);
			button.enabled = true;
		}
	}

	private async addFolder(button: Button, feedbackContainer: HTMLElement): Promise<void> {
		button.enabled = false;
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
			feedbackContainer.append(failure);
		} finally {
			if (this.active && this.element.contains(button.domNode)) button.enabled = true;
		}
	}

	private renderEntry(entry: WorkspaceTrustEntry, revision: number, list: HTMLElement): HTMLElement {
		const document = this.element.ownerDocument;
		const item = h(document, "article");
		item.className = "zeta-workspace-trust-entry";
		setSettingsItemIdentity(item, `workspaceTrust.entries.${entry.workspace}`, "resource");
		const copy = h(document, "div");
		copy.className = "zeta-workspace-trust-entry-copy";
		const path = h(document, "h5");
		path.textContent = entry.root ?? "Unknown folder (legacy trust record)";
		path.title = entry.root ?? entry.workspace;
		const status = h(document, "p");
		status.className = "zeta-workspace-trust-status trusted";
		status.textContent = "Trusted";
		const identity = h(document, "p");
		identity.className = "zeta-workspace-trust-identity";
		identity.textContent = entry.workspace;
		copy.append(path, status, identity);

		const actions = h(document, "div");
		actions.className = "zeta-workspace-trust-entry-actions";
		const forget = this.rendered.add(new Button(actions, {
			label: "Revoke Trust",
			presentation: "danger",
			onClick: () => void this.forgetEntry(entry, revision, item, forget, list),
		}));
		item.append(copy, actions);
		return item;
	}

	private async forgetEntry(entry: WorkspaceTrustEntry, revision: number, item: HTMLElement, button: Button, list: HTMLElement): Promise<void> {
		const confirmed = await this.dialogService.confirm({
			title: "Revoke Workspace Trust?",
			message: entry.root ?? entry.workspace,
			detail: "The folder will use Restricted Mode until you explicitly trust it again.",
			primaryButton: "Revoke Trust",
			cancelButton: "Cancel",
		});
		if (!confirmed) return;
		item.setAttribute("aria-busy", "true");
		button.enabled = false;
		try {
			await this.service.forget(entry.workspace, revision);
			await this.load();
		} catch (error) {
			button.enabled = true;
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

function currentWorkspaceRoot(workspace: ReturnType<IWorkspaceContextService["getWorkspace"]> | undefined): { label: string; root: string } | undefined {
	if (!workspace) return undefined;
	if (workspace.folders.length !== 1) return undefined;
	const uri = workspace.folders[0]?.uri;
	if (!uri || uri.scheme !== "file") return undefined;
	return { label: "Folder", root: uri.fsPath };
}
