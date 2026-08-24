import './media/modelCatalogItems.css';
import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import { h } from '../../../../base/browser/dom.js';
import { Button } from '../../../../base/browser/ui/button/button.js';
import { Switch } from '../../../../base/browser/ui/toggle/toggle.js';
import { Emitter, type Event } from '../../../../base/common/event.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import type { ModelRef } from '../../../../sessions/services/sessions/common/session.js';
import type { IAccountService } from '../../../../platform/accounts/common/accountService.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import { modelAccessLabel, type ModelCatalogEntry, modelRefIdentity } from '../../../services/chat/common/modelCatalog.js';
import { settingsResourceItemId } from './settingsLayout.js';
import type { SettingsItemContribution, SettingsItemView, SettingsSectionContribution } from './settingsContributions.js';
import { SettingsItemActions } from './settingsItemActions.js';
import { SubscriptionAccountCard } from './subscriptionAccountCard.js';

export interface ModelCatalogSource {
	readonly onDidChangeModels: Event<void>;
	listModelCatalog(): Promise<readonly ModelCatalogEntry[]>;
	refreshModels(): Promise<readonly ModelCatalogEntry[]>;
	isModelVisible(model: ModelRef): boolean;
	setModelVisible(model: ModelRef, visible: boolean): Promise<void>;
}

interface ModelCatalogContributionOptions {
	readonly clipboardService: IClipboardService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly models: ModelCatalogSource;
	readonly accounts: IAccountService;
	readonly onStatus: (message: string, isError: boolean) => void;
}

interface ModelItemContribution extends SettingsItemContribution {
	readonly entry: ModelCatalogEntry;
	readonly owner: ModelCatalogContribution;
}

/** Contributes account and model resources to the unified Settings tree. */
export class ModelCatalogContribution extends DisposableOwner implements SettingsSectionContribution {
	public readonly sectionId = 'models';
	private readonly changeEmitter = this.own(new Emitter<void>());
	public readonly onDidChange = this.changeEmitter.event;
	private catalog: readonly ModelCatalogEntry[] = [];
	private loadGeneration = 0;
	private status = 'Loading models…';

	constructor(public readonly options: ModelCatalogContributionOptions) {
		super();
		this.own(options.models.onDidChangeModels(() => void this.reload(false)));
		void this.reload(false);
	}

	public get groups() {
		return [
			{
				id: 'accounts',
				title: 'Provider accounts',
				description: 'Connect subscriptions that provide local model access.',
				settings: [
					this.accountItem('openai-chatgpt', 'ChatGPT subscription', 'ChatGPT', 'Use your ChatGPT subscription locally. OAuth credentials stay in Zeta’s SecretStore.', { type: 'openAiChatGptDeviceCode' }),
					this.accountItem('kimi', 'Kimi subscription', 'Kimi', 'Use your Kimi subscription locally. OAuth credentials stay in Zeta’s SecretStore.', { type: 'kimiDeviceCode' }),
				],
			},
			{
				id: 'catalog',
				title: 'Model catalog',
				description: 'Choose which available models appear in model pickers.',
				settings: [this.catalogControlItem(), ...this.catalog.map(entry => this.modelItem(entry))],
			},
		];
	}

	public refresh(): void {
		void this.reload(true);
	}

	public async updateVisibility(entry: ModelCatalogEntry, toggle: Switch, visible: boolean): Promise<void> {
		toggle.busy = true;
		this.options.onStatus('', false);
		try {
			await this.options.models.setModelVisible(entry.model, visible);
		} catch (error) {
			toggle.checked = !visible;
			this.options.onStatus(error instanceof Error ? `Unable to save model visibility: ${error.message}` : 'Unable to save model visibility.', true);
		} finally {
			toggle.busy = false;
		}
	}

	private async reload(refresh: boolean): Promise<void> {
		const generation = ++this.loadGeneration;
		if (refresh || this.catalog.length === 0) {
			this.status = 'Loading models…';
			this.changeEmitter.fire();
		}
		try {
			const catalog = await (refresh ? this.options.models.refreshModels() : this.options.models.listModelCatalog());
			if (generation !== this.loadGeneration || this.isDisposed) return;
			this.catalog = catalog;
			this.status = `${catalog.length} models`;
			this.changeEmitter.fire();
		} catch (error) {
			if (generation !== this.loadGeneration || this.isDisposed) return;
			this.status = error instanceof Error ? `Unable to refresh models: ${error.message}` : 'Unable to refresh models.';
			this.changeEmitter.fire();
		}
	}

	private accountItem(providerId: string, title: string, productName: string, signedOutCopy: string, loginMethod: { readonly type: 'openAiChatGptDeviceCode' | 'kimiDeviceCode' }): SettingsItemContribution {
		return {
			id: `models.account.${providerId}`,
			title,
			description: signedOutCopy,
			keywords: [providerId, productName],
			createView: document => {
				const host = h(document, 'div');
				return new SubscriptionAccountCard(host, this.options.accounts, { providerId, title, productName, signedOutCopy, loginMethod });
			},
		};
	}

	private catalogControlItem(): SettingsItemContribution {
		const owner = this;
		return {
			id: 'models.catalog.refresh',
			title: 'Refresh model catalog',
			description: this.status,
			keywords: ['refresh', this.status],
			createView: document => new ModelCatalogControlView(document, owner, this.status),
		};
	}

	private modelItem(entry: ModelCatalogEntry): ModelItemContribution {
		const identity = modelRefIdentity(entry.model);
		const item: ModelItemContribution = {
			id: settingsResourceItemId('models', entry.model.provider, entry.model.model),
			title: entry.displayName,
			description: `${modelAccessLabel(entry)} · ${identity}`,
			keywords: [entry.model.provider, entry.model.model, modelAccessLabel(entry)],
			entry,
			owner: this,
			createView: document => new ModelCatalogItemView(document, item, this.options),
		};
		return item;
	}
}

class ModelCatalogControlView extends DisposableOwner implements SettingsItemView {
	public readonly element: HTMLElement;
	private readonly status: HTMLParagraphElement;

	constructor(document: Document, private readonly owner: ModelCatalogContribution, status: string) {
		super();
		this.element = h(document, 'div');
		this.element.className = 'zeta-model-settings-toolbar';
		this.status = h(document, 'p');
		this.status.className = 'zeta-model-settings-status';
		this.status.textContent = status;
		this.element.append(this.status);
		const refresh = this.own(new Button(this.element, {
			label: 'Refresh',
			presentation: 'secondary',
			onClick: () => owner.refresh(),
		}));
		refresh.toggleClassName('zeta-model-settings-refresh', true);
	}

	public update(item: SettingsItemContribution): void {
		this.status.textContent = item.description;
	}
}

class ModelCatalogItemView extends DisposableOwner implements SettingsItemView {
	public readonly element: HTMLElement;
	private readonly title: HTMLHeadingElement;
	private readonly access: HTMLSpanElement;
	private readonly toggle: Switch;
	private item: ModelItemContribution;

	constructor(document: Document, item: ModelItemContribution, options: ModelCatalogContributionOptions) {
		super();
		this.item = item;
		this.element = h(document, 'article');
		this.element.className = 'zeta-model-settings-row';
		const copy = h(document, 'div');
		copy.className = 'zeta-model-settings-copy';
		const heading = h(document, 'div');
		heading.className = 'zeta-model-settings-heading';
		this.title = h(document, 'h4');
		this.access = h(document, 'span');
		this.access.className = 'zeta-model-settings-access-badge';
		heading.append(this.title, this.access);
		copy.append(heading);
		const control = h(document, 'span');
		control.className = 'zeta-model-settings-control';
		this.toggle = this.own(new Switch(control, { ariaLabel: `Show ${item.entry.displayName} in the model picker` }));
		this.own(this.toggle.onDidChange(visible => void this.item.owner.updateVisibility(this.item.entry, this.toggle, visible)));
		this.own(new SettingsItemActions(this.element, {
			label: item.entry.displayName,
			reference: {
				id: item.id,
				isDefault: () => options.models.isModelVisible(this.item.entry.model),
				reset: async () => {
					if (options.models.isModelVisible(this.item.entry.model)) return;
					this.toggle.checked = true;
					await this.item.owner.updateVisibility(this.item.entry, this.toggle, true);
				},
			},
			contextMenuProvider: options.contextMenuProvider,
			clipboardService: options.clipboardService,
			onError: error => options.onStatus(error instanceof Error ? error.message : 'Unable to run the setting action.', true),
		}));
		this.element.append(copy, control);
		this.update(item);
	}

	public update(item: SettingsItemContribution): void {
		if (!isModelItem(item)) throw new TypeError(`Model Settings item '${item.id}' changed renderer kind`);
		this.item = item;
		this.title.textContent = item.entry.displayName;
		this.access.textContent = modelAccessLabel(item.entry);
		this.toggle.setAriaLabel(`Show ${item.entry.displayName} in the model picker`);
		this.toggle.checked = item.owner.options.models.isModelVisible(item.entry.model);
	}
}

function isModelItem(item: SettingsItemContribution): item is ModelItemContribution {
	return 'entry' in item && 'owner' in item;
}
