import './media/settingsEditor.css';
import { addDisposableListener, h, stopEvent } from '../../../../base/browser/dom.js';
import { Button } from '../../../../base/browser/ui/button/button.js';
import { InputBox } from '../../../../base/browser/ui/inputbox/inputbox.js';
import { ScrollableElement } from '../../../../base/browser/ui/scrollbar/scrollableElement.js';
import { DisposableOwner, DisposableSlot } from '../../../../base/common/lifecycle.js';
import type { ILocalizationService } from '../../../services/localization/common/localizationService.js';
import type { ISettingsService } from '../../../services/preferences/common/settings.js';
import { getSettingsSection, SettingsSections, type SettingsSectionDescriptor } from '../common/settingsSections.js';
import type { SettingsPane, SettingsPaneRegistry } from './settingsPaneRegistry.js';

export interface SettingsEditorOptions {
	readonly localizationService: ILocalizationService;
	readonly paneRegistry: SettingsPaneRegistry;
	readonly settingsService: ISettingsService;
}

let nextSettingsEditorId = 1;

/** Search, navigation, and active-pane lifecycle hosted by the Workbench modal editor. */
export class SettingsEditor extends DisposableOwner {
	public readonly element: HTMLDivElement;
	private readonly activePane = this.own(new DisposableSlot<SettingsPane>());
	private readonly content: HTMLElement;
	private readonly contentDescription: HTMLParagraphElement;
	private readonly contentHeading: HTMLHeadingElement;
	private readonly contentScrollable: ScrollableElement;
	private readonly navigationEmpty: HTMLParagraphElement;
	private readonly navigationItems = new Map<string, Button>();
	private readonly navigationScrollable: ScrollableElement;
	private readonly searchInput: InputBox;
	private readonly sectionContent: HTMLDivElement;

	constructor(container: HTMLElement, private readonly options: SettingsEditorOptions) {
		super();
		for (const section of SettingsSections) {
			if (!options.paneRegistry.has(section.id)) throw new Error(`Settings section '${section.id}' has no pane registration`);
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
		const navigationList = h(ownerDocument, 'ul');
		navigationList.className = 'zeta-settings-navigation-list';
		navigationList.id = `${editorId}-navigation`;
		for (const section of SettingsSections) {
			const item = h(ownerDocument, 'li');
			const button = this.own(new Button(item, {
				label: this.localizedSectionLabel(section),
				onClick: () => this.options.settingsService.open(section.id),
			}));
			button.toggleClassName('zeta-settings-navigation-item', true);
			button.domNode.dataset.settingsSectionId = section.id;
			this.navigationItems.set(section.id, button);
			this.own(addDisposableListener(button.domNode, 'keydown', (event: KeyboardEvent) => this.handleNavigationKeydown(event, section.id)));
			navigationList.append(item);
		}
		this.navigationEmpty = h(ownerDocument, 'p');
		this.navigationEmpty.className = 'zeta-settings-navigation-empty';
		this.navigationEmpty.textContent = this.localized('chrome.noResults', 'No settings found.');
		this.navigationEmpty.setAttribute('role', 'status');
		this.navigationEmpty.hidden = true;
		this.navigationScrollable.append(navigationList, this.navigationEmpty);
		navigation.append(this.navigationScrollable.element);

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
		contentInner.append(this.contentHeading, this.contentDescription, this.sectionContent);
		this.contentScrollable.append(contentInner);
		this.content.append(this.contentScrollable.element);

		layout.append(navigation, this.content);
		this.element.append(search, layout);
		this.renderSection(getSettingsSection(this.options.settingsService.activeSectionId));

		this.own(this.options.localizationService.onDidChange(() => this.updateLocalizedChrome()));
		this.own(this.options.settingsService.onDidChangeActiveSection(sectionId => this.renderSection(getSettingsSection(sectionId))));
		this.own(this.searchInput.onDidChange(value => {
			this.filterNavigation(value);
			this.activePane.value?.setQuery?.(value);
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
		this.activePane.value?.cancelPendingChanges?.();
	}

	private renderSection(section: SettingsSectionDescriptor): void {
		for (const [sectionId, item] of this.navigationItems) {
			const active = sectionId === section.id;
			item.toggleClassName('is-active', active);
			if (active) item.domNode.setAttribute('aria-current', 'page');
			else item.domNode.removeAttribute('aria-current');
		}
		this.content.dataset.activeSettingsSection = section.id;
		this.contentHeading.textContent = this.localizedSectionLabel(section);
		this.contentDescription.textContent = this.localizedSectionDescription(section);
		this.activePane.clear();
		this.sectionContent.replaceChildren();
		const pane = this.options.paneRegistry.create(section.id, this.sectionContent);
		this.activePane.replace(pane);
		if (pane.element.parentElement !== this.sectionContent) this.sectionContent.replaceChildren(pane.element);
		pane.setQuery?.(this.searchInput.value);
		pane.activate?.();
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
		const firstVisible = this.visibleNavigationSections()[0];
		if (!firstVisible) return;
		stopEvent(event);
		this.navigationItems.get(firstVisible.id)?.focus();
	}

	private handleNavigationKeydown(event: KeyboardEvent, sectionId: string): void {
		const visibleSections = this.visibleNavigationSections();
		const currentIndex = visibleSections.findIndex(section => section.id === sectionId);
		let targetIndex: number | undefined;
		if (event.key === 'ArrowUp') targetIndex = Math.max(0, currentIndex - 1);
		else if (event.key === 'ArrowDown') targetIndex = Math.min(visibleSections.length - 1, currentIndex + 1);
		else if (event.key === 'Home') targetIndex = 0;
		else if (event.key === 'End') targetIndex = visibleSections.length - 1;
		if (targetIndex === undefined || targetIndex === currentIndex) return;
		stopEvent(event);
		this.navigationItems.get(visibleSections[targetIndex].id)?.focus();
	}

	private filterNavigation(value: string): void {
		const query = value.trim().toLocaleLowerCase();
		let matches = 0;
		for (const section of SettingsSections) {
			const visible = !query || `${this.localizedSectionLabel(section)} ${this.localizedSectionDescription(section)}`.toLocaleLowerCase().includes(query);
			const item = this.navigationItems.get(section.id)?.domNode.parentElement;
			if (item) item.hidden = !visible;
			if (visible) matches += 1;
		}
		this.navigationEmpty.hidden = matches !== 0;
		this.navigationScrollable.scrollTo(0, 0);
		this.navigationScrollable.layout();
	}

	private updateLocalizedChrome(): void {
		const searchLabel = this.localized('chrome.search', 'Search settings');
		this.searchInput.placeholder = searchLabel;
		this.searchInput.inputElement.setAttribute('aria-label', searchLabel);
		this.navigationEmpty.textContent = this.localized('chrome.noResults', 'No settings found.');
		for (const section of SettingsSections) {
			const button = this.navigationItems.get(section.id);
			if (button) button.label = this.localizedSectionLabel(section);
		}
		const section = getSettingsSection(this.options.settingsService.activeSectionId);
		this.contentHeading.textContent = this.localizedSectionLabel(section);
		this.contentDescription.textContent = this.localizedSectionDescription(section);
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

	private visibleNavigationSections(): readonly SettingsSectionDescriptor[] {
		return SettingsSections.filter(section => {
			const item = this.navigationItems.get(section.id)?.domNode.parentElement;
			return item ? !item.hidden : false;
		});
	}
}
