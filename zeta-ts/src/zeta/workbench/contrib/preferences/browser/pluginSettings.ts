import { addDisposableListener, h, fragment as createFragment } from "../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { IPluginService, PluginCatalogView, PluginPackageView } from "../../../../platform/plugins/common/pluginService.js";
import "./media/connectorSettings.css";

/** Manages activation state for legacy Plugin installations.
 *
 * Package discovery and installation belong to the generic Marketplace settings surface.
 */
export class PluginSettingsPane extends DisposableOwner {
	readonly element: HTMLDivElement;
	private readonly document: Document;
	private readonly rows = this.own(new ResettableDisposableGroup());
	private loadGeneration = 0;

	constructor(container: HTMLElement, private readonly plugins: IPluginService) {
		super();
		this.document = container.ownerDocument;
		this.element = h(this.document, "div");
		this.element.className = "zeta-integration-settings";
		container.append(this.element);
		this.own(plugins.onDidChange(() => void this.reload()));
		void this.reload();
		this.defer(() => this.element.remove());
	}

	private async reload(): Promise<void> {
		const loadGeneration = ++this.loadGeneration;
		const loading = h(this.document, "p");
		loading.className = "zeta-settings-message";
		loading.textContent = "Loading installed plugins…";
		this.element.replaceChildren(loading);
		const catalog = await this.plugins.list().catch((error: unknown) => {
			if (loadGeneration !== this.loadGeneration) return undefined;
			loading.textContent = error instanceof Error ? `Unable to load plugins: ${error.message}` : "Unable to load plugins.";
			return undefined;
		});
		if (!catalog || loadGeneration !== this.loadGeneration) return;
		this.render(catalog);
	}

	private render(catalog: PluginCatalogView): void {
		this.rows.clear();
		if (catalog.packages.length === 0) {
			const empty = h(this.document, "p");
			empty.className = "zeta-settings-message";
			empty.textContent = "No legacy plugins are installed. Discover new packages in Marketplace.";
			this.element.replaceChildren(empty);
			return;
		}
		const fragment = createFragment(this.document);
		const introduction = h(this.document, "p");
		introduction.className = "zeta-settings-message";
		introduction.textContent = "Plugin installation is managed in Marketplace. This section controls activation and grants for existing Plugin packages.";
		fragment.append(introduction);
		for (const plugin of catalog.packages) fragment.append(this.pluginCard(catalog, plugin));
		this.element.replaceChildren(fragment);
	}

	private pluginCard(catalog: PluginCatalogView, plugin: PluginPackageView): HTMLElement {
		const card = h(this.document, "section");
		card.className = "zeta-integration-card";
		const heading = h(this.document, "div");
		heading.className = "zeta-integration-heading";
		const title = h(this.document, "h4");
		title.textContent = `${plugin.id} · ${plugin.version}`;
		const state = h(this.document, "span");
		state.className = `zeta-integration-state is-${plugin.effective ? "connected" : "disconnected"}`;
		state.textContent = status(plugin);
		heading.append(title, state);
		const feedback = h(this.document, "p");
		feedback.className = "zeta-integration-feedback";
		feedback.setAttribute("role", "status");
		card.append(heading);
		if (!plugin.granted) card.append(this.action("Grant", () => this.plugins.grant(plugin, catalog.revision), feedback));
		if (plugin.granted) card.append(this.action("Revoke grant", () => this.plugins.revokeGrant(plugin, catalog.revision), feedback));
		if (!plugin.enabled) card.append(this.action("Enable", () => this.plugins.enable(plugin, catalog.revision), feedback));
		if (plugin.enabled) card.append(this.action("Disable", () => this.plugins.disable(plugin, catalog.revision), feedback));
		if (!plugin.enabled && !plugin.granted) card.append(this.action("Remove legacy installation", () => this.plugins.uninstall(plugin, catalog.revision), feedback, true));
		card.append(feedback);
		return card;
	}

	private action(label: string, invoke: () => Promise<void>, feedback: HTMLElement, danger = false): HTMLButtonElement {
		const button = h(this.document, "button");
		button.type = "button";
		button.className = `zeta-theme-action${danger ? " is-danger" : ""}`;
		button.textContent = label;
		this.rows.add(addDisposableListener(button, "click", () => {
			button.disabled = true;
			feedback.textContent = `${label}…`;
			void invoke().then(() => this.reload()).catch((error: unknown) => {
				button.disabled = false;
				feedback.textContent = error instanceof Error ? `${label} failed: ${error.message}` : `${label} failed.`;
			});
		}));
		return button;
	}
}

function status(plugin: PluginPackageView): string {
	if (plugin.revoked) return "Revoked";
	if (plugin.effective) return "Active";
	if (plugin.enabled && !plugin.granted) return "Enabled · grant required";
	if (!plugin.enabled && plugin.granted) return "Granted · disabled";
	return "Installed";
}
