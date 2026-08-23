import './media/modelSettings.css';
import { addDisposableListener, fragment as createFragment, h } from '../../../../base/browser/dom.js';
import { Switch } from '../../../../base/browser/ui/toggle/toggle.js';
import type { Event } from '../../../../base/common/event.js';
import { DisposableOwner, ResettableDisposableGroup } from '../../../../base/common/lifecycle.js';
import type { ModelRef } from '../../../../sessions/services/sessions/common/session.js';
import { modelAccessLabel, type ModelCatalogEntry } from '../../../services/chat/common/modelCatalog.js';

export interface ModelSettingsCatalog {
	readonly onDidChangeModels: Event<void>;
	listModelCatalog(): Promise<readonly ModelCatalogEntry[]>;
	refreshModels(): Promise<readonly ModelCatalogEntry[]>;
	isModelVisible(model: ModelRef): boolean;
	setModelVisible(model: ModelRef, visible: boolean): Promise<void>;
}

/** Settings projection for model discovery and picker visibility. */
export class ModelSettingsPane extends DisposableOwner {
	public readonly element: HTMLDivElement;
	private readonly document: Document;
	private readonly searchInput: HTMLInputElement;
	private readonly refreshButton: HTMLButtonElement;
	private readonly list: HTMLDivElement;
	private readonly status: HTMLParagraphElement;
	private readonly rows = this.own(new ResettableDisposableGroup());
	private catalog: readonly ModelCatalogEntry[] = [];
	private query = '';
	private loadGeneration = 0;
	private disposed = false;

	constructor(container: HTMLElement, private readonly models: ModelSettingsCatalog) {
		super();
		this.document = container.ownerDocument;
		this.element = h(this.document, 'div');
		this.element.className = 'zeta-model-settings';
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
		this.refreshButton = h(this.document, 'button');
		this.refreshButton.type = 'button';
		this.refreshButton.className = 'zeta-model-settings-refresh';
		this.refreshButton.textContent = 'Refresh';
		toolbar.append(searchLabel, this.refreshButton);
		this.list = h(this.document, 'div');
		this.list.className = 'zeta-model-settings-list';
		this.status = h(this.document, 'p');
		this.status.className = 'zeta-model-settings-status';
		this.status.setAttribute('role', 'status');
		this.status.setAttribute('aria-live', 'polite');
		this.element.append(toolbar, this.list, this.status);
		container.append(this.element);
		this.own(addDisposableListener(this.searchInput, 'input', () => {
			this.query = this.searchInput.value.trim().toLocaleLowerCase();
			this.render();
		}));
		this.own(addDisposableListener(this.refreshButton, 'click', () => void this.reload(true)));
		this.own(models.onDidChangeModels(() => void this.reload(false)));
		void this.reload(false);
		this.defer(() => {
			this.disposed = true;
			this.loadGeneration++;
			this.element.remove();
		});
	}

	private async reload(refresh: boolean): Promise<void> {
		if (this.disposed) return;
		const generation = ++this.loadGeneration;
		this.refreshButton.disabled = true;
		if (this.catalog.length === 0) this.showStatus('Loading models…', false);
		try {
			const catalog = await (refresh ? this.models.refreshModels() : this.models.listModelCatalog());
			if (generation !== this.loadGeneration) return;
			this.catalog = catalog;
			this.render();
			this.showStatus(`${catalog.length} models`, false);
		} catch (error) {
			if (generation !== this.loadGeneration) return;
			this.render();
			this.showStatus(error instanceof Error ? `Unable to refresh models: ${error.message}` : 'Unable to refresh models.', true);
		} finally {
			if (generation === this.loadGeneration) this.refreshButton.disabled = false;
		}
	}

	private render(): void {
		this.rows.clear();
		const entries = this.catalog.filter(entry => this.matchesQuery(entry));
		if (entries.length === 0) {
			const empty = h(this.document, 'p');
			empty.className = 'zeta-model-settings-empty';
			empty.textContent = this.catalog.length === 0 ? 'No models are in the Zeta catalog.' : 'No models match this filter.';
			this.list.replaceChildren(empty);
			return;
		}
		const fragment = createFragment(this.document);
		for (const entry of entries) fragment.append(this.renderModel(entry));
		this.list.replaceChildren(fragment);
	}

	private renderModel(entry: ModelCatalogEntry): HTMLElement {
		const row = h(this.document, 'article');
		row.className = 'zeta-model-settings-row';
		const copy = h(this.document, 'div');
		copy.className = 'zeta-model-settings-copy';
		const title = h(this.document, 'h4');
		title.textContent = entry.displayName;
		const access = h(this.document, 'span');
		access.className = 'zeta-model-settings-access-badge';
		access.textContent = modelAccessLabel(entry.access);
		const identity = h(this.document, 'p');
		identity.textContent = `${entry.model.provider} / ${entry.model.model}`;
		const heading = h(this.document, 'div');
		heading.className = 'zeta-model-settings-heading';
		heading.append(title, access);
		copy.append(heading, identity);
		const control = h(this.document, 'span');
		control.className = 'zeta-model-settings-control';
		const toggle = this.rows.add(new Switch(control, {
			ariaLabel: `Show ${entry.displayName} in the model picker`,
			checked: this.models.isModelVisible(entry.model),
		}));
		this.rows.add(toggle.onDidChange(visible => void this.updateVisibility(entry, toggle, visible)));
		row.append(copy, control);
		return row;
	}

	private async updateVisibility(entry: ModelCatalogEntry, toggle: Switch, visible: boolean): Promise<void> {
		toggle.enabled = false;
		this.showStatus('Saving model visibility…', false);
		try {
			await this.models.setModelVisible(entry.model, visible);
			this.showStatus('Model visibility saved.', false);
		} catch (error) {
			toggle.checked = !visible;
			this.showStatus(error instanceof Error ? `Unable to save model visibility: ${error.message}` : 'Unable to save model visibility.', true);
		} finally {
			toggle.enabled = true;
		}
	}

	private matchesQuery(entry: ModelCatalogEntry): boolean {
		if (!this.query) return true;
		return entry.displayName.toLocaleLowerCase().includes(this.query)
			|| entry.model.provider.toLocaleLowerCase().includes(this.query)
			|| entry.model.model.toLocaleLowerCase().includes(this.query)
			|| modelAccessLabel(entry.access).toLocaleLowerCase().includes(this.query);
	}

	private showStatus(message: string, isError: boolean): void {
		this.status.textContent = message;
		this.status.classList.toggle('is-error', isError);
	}
}
