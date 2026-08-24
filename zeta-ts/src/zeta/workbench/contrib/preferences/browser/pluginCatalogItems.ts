import './media/integrationItems.css';
import { h } from '../../../../base/browser/dom.js';
import { Button } from '../../../../base/browser/ui/button/button.js';
import { Emitter } from '../../../../base/common/event.js';
import { DisposableOwner, ResettableDisposableGroup } from '../../../../base/common/lifecycle.js';
import type { IPluginService, PluginCatalogView, PluginPackageView } from '../../../../platform/plugins/common/pluginService.js';
import type { SettingsItemContribution, SettingsItemView, SettingsSectionContribution } from './settingsContributions.js';
import { setSettingsItemIdentity } from './settingsItem.js';
import { settingsResourceItemId } from './settingsLayout.js';

interface PluginItemContribution extends SettingsItemContribution {
	readonly catalog: PluginCatalogView;
	readonly plugin: PluginPackageView;
	readonly owner: PluginCatalogContribution;
}

/** Contributes installed Plugin resources without owning a category page. */
export class PluginCatalogContribution extends DisposableOwner implements SettingsSectionContribution {
	public readonly sectionId = 'plugins';
	private readonly changeEmitter = this.own(new Emitter<void>());
	public readonly onDidChange = this.changeEmitter.event;
	private catalog: PluginCatalogView | undefined;
	private message = 'Loading installed plugins…';
	private loadGeneration = 0;

	constructor(public readonly plugins: IPluginService, public readonly onStatus: (message: string, isError: boolean) => void) {
		super();
		this.own(plugins.onDidChange(() => void this.reload()));
		void this.reload();
	}

	public get groups() {
		const items: SettingsItemContribution[] = [this.introductionItem()];
		if (!this.catalog || this.catalog.packages.length === 0) items.push(this.messageItem());
		if (this.catalog) items.push(...this.catalog.packages.map(plugin => this.pluginItem(this.catalog!, plugin)));
		return [{
			id: 'installed',
			title: 'Installed plugins',
			description: 'Control activation and reviewed grants for installed Plugin packages.',
			settings: items,
		}];
	}

	public async reload(): Promise<void> {
		const generation = ++this.loadGeneration;
		try {
			this.catalog = await this.plugins.list();
			if (generation !== this.loadGeneration || this.isDisposed) return;
			this.message = this.catalog.packages.length === 0
				? 'No legacy plugins are installed. Discover new packages in Marketplace.'
				: `${this.catalog.packages.length} installed plugins`;
			this.changeEmitter.fire();
		} catch (error) {
			if (generation !== this.loadGeneration || this.isDisposed) return;
			this.catalog = undefined;
			this.message = error instanceof Error ? `Unable to load plugins: ${error.message}` : 'Unable to load plugins.';
			this.changeEmitter.fire();
		}
	}

	private introductionItem(): SettingsItemContribution {
		return informationItem(
			'plugins.info.management',
			'Package management',
			'Plugin installation is managed in Marketplace. This category controls activation and grants for installed packages.',
		);
	}

	private messageItem(): SettingsItemContribution {
		return informationItem('plugins.status', 'Plugin status', this.message);
	}

	private pluginItem(catalog: PluginCatalogView, plugin: PluginPackageView): PluginItemContribution {
		const item: PluginItemContribution = {
			id: settingsResourceItemId('plugins', plugin.id, plugin.version),
			title: `${plugin.id} · ${plugin.version}`,
			description: status(plugin),
			keywords: [plugin.id, plugin.version, status(plugin)],
			catalog,
			plugin,
			owner: this,
			createView: document => new PluginCatalogItemView(document, item),
		};
		return item;
	}
}

class PluginCatalogItemView extends DisposableOwner implements SettingsItemView {
	public readonly element: HTMLElement;
	private readonly bindings = this.own(new ResettableDisposableGroup());

	constructor(document: Document, item: PluginItemContribution) {
		super();
		this.element = h(document, 'section');
		this.element.className = 'zeta-integration-card';
		this.update(item);
	}

	public update(item: SettingsItemContribution): void {
		if (!isPluginItem(item)) throw new TypeError(`Plugin Settings item '${item.id}' changed renderer kind`);
		this.bindings.clear();
		this.element.replaceChildren();
		setSettingsItemIdentity(this.element, item.id, 'resource');
		const heading = h(this.element.ownerDocument, 'div');
		heading.className = 'zeta-integration-heading';
		const title = h(this.element.ownerDocument, 'h4');
		title.textContent = `${item.plugin.id} · ${item.plugin.version}`;
		const state = h(this.element.ownerDocument, 'span');
		state.className = `zeta-integration-state is-${item.plugin.effective ? 'connected' : 'disconnected'}`;
		state.textContent = status(item.plugin);
		heading.append(title, state);
		const feedback = h(this.element.ownerDocument, 'p');
		feedback.className = 'zeta-integration-feedback';
		feedback.setAttribute('role', 'status');
		this.element.append(heading);
		if (!item.plugin.granted) this.action(item, 'Grant', () => item.owner.plugins.grant(item.plugin, item.catalog.revision), feedback);
		if (item.plugin.granted) this.action(item, 'Revoke grant', () => item.owner.plugins.revokeGrant(item.plugin, item.catalog.revision), feedback);
		if (!item.plugin.enabled) this.action(item, 'Enable', () => item.owner.plugins.enable(item.plugin, item.catalog.revision), feedback);
		if (item.plugin.enabled) this.action(item, 'Disable', () => item.owner.plugins.disable(item.plugin, item.catalog.revision), feedback);
		if (!item.plugin.enabled && !item.plugin.granted) this.action(item, 'Remove legacy installation', () => item.owner.plugins.uninstall(item.plugin, item.catalog.revision), feedback, true);
		this.element.append(feedback);
	}

	private action(item: PluginItemContribution, label: string, invoke: () => Promise<void>, feedback: HTMLElement, danger = false): void {
		const button = this.bindings.add(new Button(this.element, {
			label,
			presentation: danger ? 'danger' : 'secondary',
			onClick: () => {
				button.enabled = false;
				feedback.textContent = `${label}…`;
				void invoke().then(() => item.owner.reload()).catch((error: unknown) => {
					button.enabled = true;
					feedback.textContent = error instanceof Error ? `${label} failed: ${error.message}` : `${label} failed.`;
					item.owner.onStatus(feedback.textContent, true);
				});
			},
		}));
	}
}

class InformationSettingsItemView extends DisposableOwner implements SettingsItemView {
	public readonly element: HTMLElement;
	private readonly heading: HTMLElement;
	private readonly copy: HTMLElement;

	constructor(document: Document, item: SettingsItemContribution) {
		super();
		this.element = h(document, 'div');
		this.element.className = 'zeta-settings-message';
		this.heading = h(document, 'h4');
		this.copy = h(document, 'p');
		this.element.append(this.heading, this.copy);
		this.update(item);
	}

	public update(item: SettingsItemContribution): void {
		this.heading.textContent = item.title;
		this.copy.textContent = item.description;
	}
}

function informationItem(id: string, title: string, description: string): SettingsItemContribution {
	const item: SettingsItemContribution = {
		id,
		title,
		description,
		createView: document => new InformationSettingsItemView(document, item),
	};
	return item;
}

function isPluginItem(item: SettingsItemContribution): item is PluginItemContribution {
	return 'plugin' in item && 'catalog' in item && 'owner' in item;
}

function status(plugin: PluginPackageView): string {
	if (plugin.revoked) return 'Revoked';
	if (plugin.effective) return 'Active';
	if (plugin.enabled && !plugin.granted) return 'Enabled · grant required';
	if (!plugin.enabled && plugin.granted) return 'Granted · disabled';
	return 'Installed';
}
