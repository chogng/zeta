import './media/settingsEditor.css';
import { h, stopEvent } from '../../../../base/browser/dom.js';
import { InputBox } from '../../../../base/browser/ui/inputbox/inputbox.js';
import { ScrollableElement } from '../../../../base/browser/ui/scrollbar/scrollableElement.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import type { ILocalizationService } from '../../../services/localization/common/localizationService.js';
import type { ISettingsService } from '../../../services/preferences/common/settings.js';
import type { SettingsContributionRegistry, SettingsItemContribution, SettingsItemView } from './settingsContributions.js';
import { getSettingsSection, SettingsSections, type SettingsSectionDescriptor, type SettingsSectionGroupDescriptor } from './settingsLayout.js';
import { SettingsTree } from './settingsTree.js';
import { SettingsTreeModel } from './settingsTreeModels.js';
import { TOCTree, TOCTreeModel, type SettingsTOCEntry } from './tocTree.js';

export interface SettingsEditorOptions {
	readonly localizationService: ILocalizationService;
	readonly contributions: SettingsContributionRegistry;
	readonly settingsService: ISettingsService;
}

let nextSettingsEditorId = 1;

/** Owns one Settings tree whose layout-derived scope changes with navigation and search. */
export class SettingsEditor extends DisposableOwner {
	public readonly element: HTMLDivElement;
	private readonly content: HTMLElement;
	private readonly contentDescription: HTMLParagraphElement;
	private readonly contentHeading: HTMLHeadingElement;
	private readonly contentStatus: HTMLParagraphElement;
	private readonly contentScrollable: ScrollableElement;
	private readonly navigationEmpty: HTMLParagraphElement;
	private readonly navigationScrollable: ScrollableElement;
	private readonly tocTree: TOCTree;
	private activeNavigationTarget: Extract<SettingsTOCEntry, { readonly kind: 'target' }> | undefined;
	private pendingNavigationTarget: Extract<SettingsTOCEntry, { readonly kind: 'target' }> | undefined;
	private readonly searchInput: InputBox;
	private readonly sectionContent: HTMLDivElement;
	private readonly settingsModel: SettingsTreeModel<SettingsItemContribution>;
	private readonly settingsTree: SettingsTree<SettingsItemContribution>;
	private readonly itemViews = new Map<string, SettingsItemView>();

	constructor(container: HTMLElement, private readonly options: SettingsEditorOptions) {
		super();
		this.own(options.contributions);
		for (const section of SettingsSections) {
			if (!options.contributions.has(section.id)) throw new Error(`Settings section '${section.id}' has no layout contribution`);
		}

		const ownerDocument = container.ownerDocument;
		const editorId = `zeta-settings-editor-${nextSettingsEditorId++}`;
		this.element = h(ownerDocument, 'div');
		this.element.className = 'zeta-settings-editor';
		container.append(this.element);

		const search = h(ownerDocument, 'div');
		search.className = 'zeta-settings-search';
		search.setAttribute('role', 'search');
		this.searchInput = this.own(new InputBox(search, {
			type: 'search',
			placeholder: this.localized('chrome.search', 'Search settings'),
			ariaLabel: this.localized('chrome.search', 'Search settings'),
			ariaControls: `${editorId}-navigation`,
		}));
		this.searchInput.element.classList.add('zeta-settings-search-input');
		search.append(this.searchInput.element);

		const layout = h(ownerDocument, 'div');
		layout.className = 'zeta-settings-layout';

		const navigation = h(ownerDocument, 'nav');
		navigation.className = 'zeta-settings-sidebar';
		navigation.setAttribute('aria-label', 'Settings categories');
		this.navigationScrollable = this.own(new ScrollableElement(navigation, {
			direction: 'vertical',
			vertical: 'auto',
			tabIndex: -1,
			wheel: { consume: 'when-scrolling' },
		}));
		this.navigationScrollable.element.classList.add('zeta-settings-sidebar-scrollable');
		this.tocTree = this.own(new TOCTree(this.navigationScrollable.contentElement, new TOCTreeModel(options.contributions), {
			ariaLabel: this.localized('chrome.categories', 'Settings categories'),
			groupLabel: group => this.localizedGroupLabel(group),
			groupDescription: group => this.localizedGroupDescription(group),
			sectionLabel: section => this.localizedSectionLabel(section),
			sectionDescription: section => this.localizedSectionDescription(section),
		}));
		this.tocTree.element.id = `${editorId}-navigation`;
		this.navigationEmpty = h(ownerDocument, 'p');
		this.navigationEmpty.className = 'zeta-settings-navigation-empty';
		this.navigationEmpty.textContent = this.localized('chrome.noResults', 'No settings found.');
		this.navigationEmpty.setAttribute('role', 'status');
		this.navigationEmpty.hidden = true;
		this.navigationScrollable.append(this.navigationEmpty);

		this.content = h(ownerDocument, 'main');
		this.content.className = 'zeta-settings-page';
		this.content.dataset.settingsContainer = '';
		this.content.tabIndex = -1;
		this.contentScrollable = this.own(new ScrollableElement(this.content, {
			direction: 'vertical',
			vertical: 'auto',
			tabIndex: -1,
			wheel: { consume: 'when-scrolling' },
		}));
		this.contentScrollable.element.classList.add('zeta-settings-page-scrollable');
		const contentInner = h(ownerDocument, 'div');
		contentInner.className = 'zeta-settings-page-inner';
		this.contentHeading = h(ownerDocument, 'h3');
		this.contentHeading.id = `${editorId}-section`;
		this.content.setAttribute('aria-labelledby', this.contentHeading.id);
		this.contentDescription = h(ownerDocument, 'p');
		this.contentDescription.className = 'zeta-settings-description';
		this.sectionContent = h(ownerDocument, 'div');
		this.sectionContent.className = 'zeta-settings-section-content';
		this.sectionContent.dataset.settingsSectionContent = '';
		this.contentStatus = h(ownerDocument, 'p');
		this.contentStatus.className = 'zeta-configuration-settings-status';
		this.contentStatus.setAttribute('role', 'status');
		this.contentStatus.setAttribute('aria-live', 'polite');
		this.contentStatus.hidden = true;
		contentInner.append(this.contentHeading, this.contentDescription, this.sectionContent, this.contentStatus);
		this.contentScrollable.append(contentInner);
		this.content.append(this.contentScrollable.element);

		layout.append(navigation, this.content);
		this.element.append(search, layout);
		const initialSection = getSettingsSection(this.options.settingsService.activeSectionId);
		this.settingsModel = this.own(new SettingsTreeModel<SettingsItemContribution>());
		this.settingsModel.setChildren(options.contributions.rootNodes);
		this.settingsModel.setNavigationTarget(initialSection.id);
		this.settingsTree = this.own(new SettingsTree(this.sectionContent, {
			model: this.settingsModel,
			rootClassName: 'zeta-settings-content-tree',
			groupClassName: 'zeta-configuration-settings-group zeta-settings-content-group',
			groupDescriptionClassName: 'zeta-configuration-settings-group-description',
			itemsClassName: 'zeta-configuration-settings-list',
			renderItem: item => this.renderSettingsItem(item.value),
			updateItem: item => this.itemViews.get(item.id)?.update?.(item.value),
			disposeItem: item => this.disposeSettingsItem(item.id),
		}));
		this.renderSection(initialSection);

		this.own(this.options.localizationService.onDidChange(() => this.updateLocalizedChrome()));
		this.own(this.options.contributions.onDidChangeSection(sectionId => {
			this.settingsModel.setNodeChildren(sectionId, this.options.contributions.getSectionChildren(sectionId));
			this.tocTree.refresh();
		}));
		this.own(this.options.contributions.onDidChangeStatus(status => {
			this.contentStatus.textContent = status.message;
			this.contentStatus.classList.toggle('is-error', status.isError);
			this.contentStatus.hidden = !status.message;
		}));
		this.own(this.options.settingsService.onDidChangeActiveSection(sectionId => this.renderSection(getSettingsSection(sectionId))));
		this.own(this.tocTree.onDidOpen(entry => this.openNavigationEntry(entry)));
		this.own(this.tocTree.onDidChangeCollapseState(({ element, collapsed }) => {
			if (element.kind !== 'group') return;
			const activeSectionId = this.options.settingsService.activeSectionId;
			const containsActiveSection = element.group.sections.some(section => section.id === activeSectionId);
			this.tocTree.setSelection([containsActiveSection && collapsed ? element.id : activeSectionId]);
		}));
		this.own(this.tocTree.onDidChangeFind(({ pattern, matches }) => {
			this.navigationEmpty.hidden = !pattern || matches.length !== 0;
		}));
		this.own(this.searchInput.onDidChange(value => {
			this.filterNavigation(value);
			this.settingsModel.setQuery(value);
		}));
		this.own(this.searchInput.onKeyDown(event => this.handleSearchKeydown(event)));
		this.defer(() => this.element.remove());
	}

	public focus(): void {
		this.searchInput.focus();
	}

	public layout(): void {
		this.navigationScrollable.layout();
		this.contentScrollable.layout();
	}

	public cancelPendingChanges(): void {
		this.options.contributions.cancelPendingChanges();
		for (const view of this.itemViews.values()) view.cancelPendingChanges?.();
	}

	private renderSection(section: SettingsSectionDescriptor): void {
		const pendingTarget = this.pendingNavigationTarget?.section.id === section.id ? this.pendingNavigationTarget : undefined;
		const navigationId = pendingTarget?.id ?? section.id;
		this.tocTree.expandTo(navigationId);
		this.tocTree.setSelection([navigationId]);
		this.content.dataset.activeSettingsSection = section.id;
		this.contentHeading.textContent = this.localizedSectionLabel(section);
		this.contentDescription.textContent = this.localizedSectionDescription(section);
		this.showNavigationTarget(section, pendingTarget);
		this.pendingNavigationTarget = undefined;
	}

	private openNavigationEntry(entry: SettingsTOCEntry): void {
		if (entry.kind === 'group') return;
		if (entry.kind === 'section') {
			this.pendingNavigationTarget = undefined;
			if (this.options.settingsService.activeSectionId === entry.section.id) {
				this.tocTree.setSelection([entry.id]);
				this.showNavigationTarget(entry.section, undefined);
				return;
			}
			this.options.settingsService.open(entry.section.id);
			return;
		}
		this.pendingNavigationTarget = entry;
		if (this.options.settingsService.activeSectionId === entry.section.id) {
			this.tocTree.expandTo(entry.id);
			this.tocTree.setSelection([entry.id]);
			this.showNavigationTarget(entry.section, entry);
			this.pendingNavigationTarget = undefined;
			return;
		}
		this.options.settingsService.open(entry.section.id);
	}

	private showNavigationTarget(
		section: SettingsSectionDescriptor,
		entry: Extract<SettingsTOCEntry, { readonly kind: 'target' }> | undefined,
	): void {
		const targetId = entry?.target.targetId ?? section.id;
		const target = this.settingsModel.getGroup(targetId);
		if (!target) throw new RangeError(`Settings layout does not expose navigation target '${targetId}'`);
		this.settingsTree.setNavigationTarget(targetId);
		this.activeNavigationTarget = entry;
		this.content.classList.toggle('has-navigation-target', entry !== undefined);
		if (entry) this.content.dataset.activeSettingsTarget = entry.target.targetId;
		else delete this.content.dataset.activeSettingsTarget;
		this.contentHeading.textContent = entry ? target.title : this.localizedSectionLabel(section);
		this.contentDescription.textContent = entry ? target.description : this.localizedSectionDescription(section);
		this.contentScrollable.scrollTo(0, 0);
		this.contentScrollable.layout();
	}

	private handleSearchKeydown(event: KeyboardEvent): void {
		if (event.key === 'Escape' && this.searchInput.value) {
			stopEvent(event);
			this.searchInput.value = '';
			return;
		}
		if (event.key !== 'ArrowDown') return;
		if (!this.tocTree.focus) return;
		stopEvent(event);
		this.tocTree.domFocus();
	}

	private filterNavigation(value: string): void {
		const query = value.trim().toLocaleLowerCase();
		this.tocTree.setFindPattern(query);
		this.navigationScrollable.scrollTo(0, 0);
		this.navigationScrollable.layout();
	}

	private updateLocalizedChrome(): void {
		const searchLabel = this.localized('chrome.search', 'Search settings');
		this.searchInput.placeholder = searchLabel;
		this.searchInput.inputElement.setAttribute('aria-label', searchLabel);
		this.navigationEmpty.textContent = this.localized('chrome.noResults', 'No settings found.');
		this.tocTree.rerender();
		const section = getSettingsSection(this.options.settingsService.activeSectionId);
		if (this.activeNavigationTarget) {
			this.contentHeading.textContent = this.activeNavigationTarget.target.label;
		} else {
			this.contentHeading.textContent = this.localizedSectionLabel(section);
			this.contentDescription.textContent = this.localizedSectionDescription(section);
		}
	}

	private renderSettingsItem(item: SettingsItemContribution): HTMLElement {
		const view = item.createView(this.element.ownerDocument);
		if (this.itemViews.has(item.id)) {
			view.dispose();
			throw new Error(`Settings item view is already rendered: ${item.id}`);
		}
		this.itemViews.set(item.id, view);
		return view.element;
	}

	private disposeSettingsItem(itemId: string): void {
		const view = this.itemViews.get(itemId);
		if (!view) return;
		view.dispose();
		this.itemViews.delete(itemId);
	}

	private localized(key: string, fallback: string): string {
		return this.options.localizationService.translate('zeta.settings', key, fallback);
	}

	private localizedSectionLabel(section: SettingsSectionDescriptor): string {
		return this.localized(`sections.${section.id}.label`, section.label);
	}

	private localizedSectionDescription(section: SettingsSectionDescriptor): string {
		return this.localized(`sections.${section.id}.description`, section.description);
	}

	private localizedGroupLabel(group: SettingsSectionGroupDescriptor): string {
		return this.localized(`groups.${group.id}.label`, group.label);
	}

	private localizedGroupDescription(group: SettingsSectionGroupDescriptor): string {
		return this.localized(`groups.${group.id}.description`, group.description);
	}
}
