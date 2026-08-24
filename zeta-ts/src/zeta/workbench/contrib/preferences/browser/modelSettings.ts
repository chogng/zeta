import './media/modelSettings.css';
import { addDisposableListener, h } from '../../../../base/browser/dom.js';
import { Button } from '../../../../base/browser/ui/button/button.js';
import { Switch } from '../../../../base/browser/ui/toggle/toggle.js';
import type { Event } from '../../../../base/common/event.js';
import { combinedDisposable, DisposableOwner, DisposableStore, type IDisposable } from '../../../../base/common/lifecycle.js';
import type { ModelRef } from '../../../../sessions/services/sessions/common/session.js';
import { modelAccessLabel, type ModelCatalogEntry, modelRefIdentity } from '../../../services/chat/common/modelCatalog.js';
import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import { SettingsItemActions } from './settingsItemActions.js';
import type { IAccountService } from '../../../../platform/accounts/common/accountService.js';
import { SubscriptionAccountCard } from './subscriptionAccountCard.js';

export interface ModelSettingsCatalog {
	readonly onDidChangeModels: Event<void>;
	listModelCatalog(): Promise<readonly ModelCatalogEntry[]>;
	refreshModels(): Promise<readonly ModelCatalogEntry[]>;
	isModelVisible(model: ModelRef): boolean;
	setModelVisible(model: ModelRef, visible: boolean): Promise<void>;
}

interface ModelSettingsPaneOptions {
	readonly clipboardService: IClipboardService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly models: ModelSettingsCatalog;
	readonly accounts: IAccountService;
}

interface ModelSettingsRowView {
	readonly entry: ModelCatalogEntry;
	readonly element: HTMLElement;
	readonly toggle: Switch;
	readonly resources: IDisposable;
}

/** Settings projection for model discovery and picker visibility. */
export class ModelSettingsPane extends DisposableOwner {
	public readonly element: HTMLDivElement;
	private readonly document: Document;
	private readonly searchInput: HTMLInputElement;
	private readonly refreshButton: Button;
	private readonly list: HTMLDivElement;
	private readonly empty: HTMLParagraphElement;
	private readonly status: HTMLParagraphElement;
	private readonly rows = this.own(new DisposableStore());
	private readonly rowViews = new Map<string, ModelSettingsRowView>();
	private catalog: readonly ModelCatalogEntry[] = [];
	private query = '';
	private loadGeneration = 0;
	private hasAcceptedCatalog = false;

	constructor(container: HTMLElement, private readonly options: ModelSettingsPaneOptions) {
		super();
		this.document = container.ownerDocument;
		this.element = h(this.document, 'div');
		this.element.className = 'zeta-model-settings';
		const chatgptAccount = this.own(new SubscriptionAccountCard(this.element, options.accounts, {
			providerId: 'openai-chatgpt',
			title: 'ChatGPT subscription',
			productName: 'ChatGPT',
			signedOutCopy: 'Use your ChatGPT subscription locally. OAuth credentials stay in Zeta’s SecretStore.',
			loginMethod: { type: 'openAiChatGptDeviceCode' },
		}));
		const kimiAccount = this.own(new SubscriptionAccountCard(this.element, options.accounts, {
			providerId: 'kimi',
			title: 'Kimi subscription',
			productName: 'Kimi',
			signedOutCopy: 'Use your Kimi subscription locally. OAuth credentials stay in Zeta’s SecretStore.',
			loginMethod: { type: 'kimiDeviceCode' },
		}));
		const toolbar = h(this.document, 'div');
		toolbar.className = 'zeta-model-settings-toolbar';
		const searchLabel = h(this.document, 'label');
		searchLabel.className = 'zeta-model-settings-search';
		const searchTitle = h(this.document, 'span');
		searchTitle.textContent = 'Filter models';
		this.searchInput = h(this.document, 'input');
		this.searchInput.type = 'search';
		this.searchInput.placeholder = 'Search models or providers';
		searchLabel.append(searchTitle, this.searchInput);
		toolbar.append(searchLabel);
		this.refreshButton = this.own(new Button(toolbar, {
			label: 'Refresh',
			presentation: 'secondary',
			onClick: () => void this.reload(true),
		}));
		this.refreshButton.toggleClassName('zeta-model-settings-refresh', true);
		this.list = h(this.document, 'div');
		this.list.className = 'zeta-model-settings-list';
		this.empty = h(this.document, 'p');
		this.empty.className = 'zeta-model-settings-empty';
		this.empty.hidden = true;
		this.list.append(this.empty);
		this.status = h(this.document, 'p');
		this.status.className = 'zeta-model-settings-status';
		this.status.setAttribute('role', 'status');
		this.status.setAttribute('aria-live', 'polite');
		this.element.append(chatgptAccount.element, kimiAccount.element, toolbar, this.list, this.status);
		container.append(this.element);
		this.own(addDisposableListener(this.searchInput, 'input', () => {
			this.query = this.searchInput.value.trim().toLocaleLowerCase();
			this.applyFilter();
		}));
		this.own(options.models.onDidChangeModels(() => void this.reload(false)));
		void this.reload(false);
		this.defer(() => {
			this.loadGeneration++;
			this.element.remove();
		});
	}

	private async reload(refresh: boolean): Promise<void> {
		if (this.isDisposed) return;
		const generation = ++this.loadGeneration;
		const showProgress = refresh || !this.hasAcceptedCatalog || !this.refreshButton.enabled;
		if (showProgress) {
			this.refreshButton.enabled = false;
			if (this.catalog.length === 0) this.showStatus('Loading models…', false);
		}
		try {
			const catalog = await (refresh ? this.options.models.refreshModels() : this.options.models.listModelCatalog());
			if (generation !== this.loadGeneration) return;
			const changed = this.acceptCatalog(catalog);
			if (showProgress || changed) this.showStatus(`${catalog.length} models`, false);
		} catch (error) {
			if (generation !== this.loadGeneration) return;
			if (!this.hasAcceptedCatalog) this.acceptCatalog([]);
			this.showStatus(error instanceof Error ? `Unable to refresh models: ${error.message}` : 'Unable to refresh models.', true);
		} finally {
			if (generation === this.loadGeneration) this.refreshButton.enabled = true;
		}
	}

	private acceptCatalog(catalog: readonly ModelCatalogEntry[]): boolean {
		if (this.hasAcceptedCatalog && sameModelCatalog(this.catalog, catalog)) {
			this.catalog = catalog;
			this.syncVisibility();
			return false;
		}
		this.hasAcceptedCatalog = true;
		this.catalog = catalog;
		const identities = new Set(catalog.map(entry => modelRefIdentity(entry.model)));
		for (const [identity, view] of this.rowViews) {
			if (identities.has(identity)) continue;
			this.disposeRow(identity, view);
		}
		for (const entry of catalog) {
			const identity = modelRefIdentity(entry.model);
			const existing = this.rowViews.get(identity);
			if (existing && sameModelEntry(existing.entry, entry)) continue;
			if (existing) this.disposeRow(identity, existing);
			this.rowViews.set(identity, this.createModelRow(entry));
		}
		this.syncVisibility();
		this.applyFilter();
		return true;
	}

	private createModelRow(entry: ModelCatalogEntry): ModelSettingsRowView {
		const row = h(this.document, 'article');
		row.className = 'zeta-model-settings-row';
		const settingId = `${entry.model.provider}/${entry.model.model}`;
		const copy = h(this.document, 'div');
		copy.className = 'zeta-model-settings-copy';
		const title = h(this.document, 'h4');
		title.textContent = entry.displayName;
		const access = h(this.document, 'span');
		access.className = 'zeta-model-settings-access-badge';
		access.textContent = modelAccessLabel(entry);
		const heading = h(this.document, 'div');
		heading.className = 'zeta-model-settings-heading';
		heading.append(title, access);
		copy.append(heading);
		const control = h(this.document, 'span');
		control.className = 'zeta-model-settings-control';
		const toggle = new Switch(control, {
			ariaLabel: `Show ${entry.displayName} in the model picker`,
			checked: this.options.models.isModelVisible(entry.model),
		});
		const changeListener = toggle.onDidChange(visible => void this.updateVisibility(entry, toggle, visible));
		const actions = new SettingsItemActions(row, {
			label: entry.displayName,
			reference: {
				id: settingId,
				isDefault: () => this.options.models.isModelVisible(entry.model),
				reset: () => this.resetVisibility(entry, toggle),
			},
			contextMenuProvider: this.options.contextMenuProvider,
			clipboardService: this.options.clipboardService,
			onError: error => this.showStatus(error instanceof Error ? error.message : 'Unable to run the setting action.', true),
		});
		row.append(copy, control);
		return {
			entry,
			element: row,
			toggle,
			resources: this.rows.add(combinedDisposable(actions, changeListener, toggle)),
		};
	}

	private disposeRow(identity: string, view: ModelSettingsRowView): void {
		view.resources.dispose();
		view.element.remove();
		this.rowViews.delete(identity);
	}

	private syncVisibility(): void {
		for (const entry of this.catalog) {
			const view = this.rowViews.get(modelRefIdentity(entry.model));
			if (view) view.toggle.checked = this.options.models.isModelVisible(entry.model);
		}
	}

	private applyFilter(): void {
		const visible = this.catalog
			.filter(entry => this.matchesQuery(entry))
			.map(entry => this.rowViews.get(modelRefIdentity(entry.model)))
			.filter((view): view is ModelSettingsRowView => view !== undefined);
		const visibleElements = new Set(visible.map(view => view.element));
		let nextNode = this.list.firstChild;
		for (const view of visible) {
			if (view.element !== nextNode) this.list.insertBefore(view.element, nextNode);
			nextNode = view.element.nextSibling;
		}
		for (const view of this.rowViews.values()) {
			if (!visibleElements.has(view.element)) view.element.remove();
		}
		this.list.append(this.empty);
		this.empty.textContent = this.catalog.length === 0 ? 'No models are in the Zeta catalog.' : 'No models match this filter.';
		this.empty.hidden = visible.length !== 0;
	}

	private async updateVisibility(entry: ModelCatalogEntry, toggle: Switch, visible: boolean): Promise<void> {
		toggle.enabled = false;
		this.showStatus('Saving model visibility…', false);
		try {
			await this.options.models.setModelVisible(entry.model, visible);
			this.showStatus('Model visibility saved.', false);
		} catch (error) {
			toggle.checked = !visible;
			this.showStatus(error instanceof Error ? `Unable to save model visibility: ${error.message}` : 'Unable to save model visibility.', true);
		} finally {
			toggle.enabled = true;
		}
	}

	private async resetVisibility(entry: ModelCatalogEntry, toggle: Switch): Promise<void> {
		if (this.options.models.isModelVisible(entry.model)) return;
		toggle.checked = true;
		await this.updateVisibility(entry, toggle, true);
	}

	private matchesQuery(entry: ModelCatalogEntry): boolean {
		if (!this.query) return true;
		return entry.displayName.toLocaleLowerCase().includes(this.query)
			|| entry.model.provider.toLocaleLowerCase().includes(this.query)
			|| entry.model.model.toLocaleLowerCase().includes(this.query)
			|| modelAccessLabel(entry).toLocaleLowerCase().includes(this.query);
	}

	private showStatus(message: string, isError: boolean): void {
		this.status.textContent = message;
		this.status.classList.toggle('is-error', isError);
	}
}

function sameModelCatalog(left: readonly ModelCatalogEntry[], right: readonly ModelCatalogEntry[]): boolean {
	return left.length === right.length && left.every((entry, index) => {
		const candidate = right[index];
		return candidate !== undefined && sameModelEntry(entry, candidate);
	});
}

function sameModelEntry(left: ModelCatalogEntry, right: ModelCatalogEntry): boolean {
	return modelRefIdentity(left.model) === modelRefIdentity(right.model)
		&& left.displayName === right.displayName
		&& left.access === right.access
		&& left.outputTransport === right.outputTransport;
}
