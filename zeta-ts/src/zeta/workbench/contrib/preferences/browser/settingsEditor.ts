import './media/settingsEditor.css';
import { h, stopEvent } from '../../../../base/browser/dom.js';
import { InputBox } from '../../../../base/browser/ui/inputbox/inputbox.js';
import { ScrollableElement } from '../../../../base/browser/ui/scrollbar/scrollableElement.js';
import { ObjectTree } from '../../../../base/browser/ui/tree/objectTree.js';
import type { ObjectTreeElement } from '../../../../base/browser/ui/tree/objectTreeModel.js';
import { TreeFindMatchType, TreeFindMode } from '../../../../base/browser/ui/tree/tree.js';
import { DisposableOwner, DisposableSlot } from '../../../../base/common/lifecycle.js';
import type { ILocalizationService } from '../../../services/localization/common/localizationService.js';
import type { ISettingsService } from '../../../services/preferences/common/settings.js';
import { getSettingsSection, SettingsSections, type SettingsNavigationTargetDescriptor, type SettingsSectionDescriptor } from '../common/settingsSections.js';
import type { SettingsPane, SettingsPaneRegistry } from './settingsPaneRegistry.js';

export interface SettingsEditorOptions {
	readonly localizationService: ILocalizationService;
	readonly paneRegistry: SettingsPaneRegistry;
	readonly settingsService: ISettingsService;
}

let nextSettingsEditorId = 1;

type SettingsNavigationEntry =
	| { readonly kind: 'section'; readonly id: string; readonly section: SettingsSectionDescriptor }
	| { readonly kind: 'target'; readonly id: string; readonly section: SettingsSectionDescriptor; readonly target: SettingsNavigationTargetDescriptor };

/** Search, navigation, and active-pane lifecycle hosted by the Workbench modal editor. */
export class SettingsEditor extends DisposableOwner {
	public readonly element: HTMLDivElement;
	private readonly activePane = this.own(new DisposableSlot<SettingsPane>());
	private readonly content: HTMLElement;
	private readonly contentDescription: HTMLParagraphElement;
	private readonly contentHeading: HTMLHeadingElement;
	private readonly contentScrollable: ScrollableElement;
	private readonly navigationEmpty: HTMLParagraphElement;
	private readonly navigationScrollable: ScrollableElement;
	private readonly navigationTree: ObjectTree<SettingsNavigationEntry>;
	private activeNavigationTarget: Extract<SettingsNavigationEntry, { readonly kind: 'target' }> | undefined;
	private pendingNavigationTarget: Extract<SettingsNavigationEntry, { readonly kind: 'target' }> | undefined;
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
		this.navigationTree = this.own(new ObjectTree(this.navigationScrollable.contentElement, {
			ariaLabel: this.localized('chrome.categories', 'Settings categories'),
			scrolling: 'external',
			expandOnlyOnTwistieClick: false,
			findMatchType: TreeFindMatchType.Contiguous,
			findMode: TreeFindMode.Filter,
			getHeight: () => 26,
			indent: 12,
			keyboardNavigationLabelProvider: {
				getKeyboardNavigationLabel: entry => entry.kind === 'section'
					? `${this.localizedSectionLabel(entry.section)} ${this.localizedSectionDescription(entry.section)}`
					: [entry.target.label, ...(entry.target.keywords ?? [])].join(' '),
			},
			modelOptions: {
				defaultCollapseState: 'collapsed',
				identityProvider: { getId: entry => entry.id },
			},
			selectionPresentation: 'subtle',
			renderElement: entry => {
				const label = h(ownerDocument, 'span');
				label.className = 'zeta-settings-navigation-label';
				if (entry.kind === 'section') label.dataset.settingsSectionId = entry.section.id;
				else label.dataset.settingsTargetId = entry.target.targetId;
				label.textContent = entry.kind === 'section' ? this.localizedSectionLabel(entry.section) : entry.target.label;
				return label;
			},
		}));
		this.navigationTree.element.classList.add('zeta-settings-navigation-tree');
		this.navigationTree.element.id = `${editorId}-navigation`;
		this.navigationTree.setChildren(settingsNavigationElements());
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
		contentInner.append(this.contentHeading, this.contentDescription, this.sectionContent);
		this.contentScrollable.append(contentInner);
		this.content.append(this.contentScrollable.element);

		layout.append(navigation, this.content);
		this.element.append(search, layout);
		this.renderSection(getSettingsSection(this.options.settingsService.activeSectionId));

		this.own(this.options.localizationService.onDidChange(() => this.updateLocalizedChrome()));
		this.own(this.options.settingsService.onDidChangeActiveSection(sectionId => this.renderSection(getSettingsSection(sectionId))));
		this.own(this.navigationTree.onDidChangeSelection(({ elements, browserEvent }) => {
			const entry = elements[0];
			if (entry && browserEvent) this.openNavigationEntry(entry);
		}));
		this.own(this.navigationTree.onDidAccept(({ element, node }) => {
			if (node.collapsible) this.navigationTree.toggleCollapsed(element.id);
			this.openNavigationEntry(element);
		}));
		this.own(this.navigationTree.onDidChangeFind(({ pattern, matches }) => {
			this.navigationEmpty.hidden = !pattern || matches.length !== 0;
		}));
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
		const pendingTarget = this.pendingNavigationTarget?.section.id === section.id ? this.pendingNavigationTarget : undefined;
		const navigationId = pendingTarget?.id ?? section.id;
		this.navigationTree.expandTo(navigationId);
		this.navigationTree.setSelection([navigationId]);
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
		this.showNavigationTarget(section, pendingTarget);
		this.pendingNavigationTarget = undefined;
	}

	private openNavigationEntry(entry: SettingsNavigationEntry): void {
		if (entry.kind === 'section') {
			this.pendingNavigationTarget = undefined;
			if (this.options.settingsService.activeSectionId === entry.section.id) {
				this.navigationTree.setSelection([entry.id]);
				this.showNavigationTarget(entry.section, undefined);
				return;
			}
			this.options.settingsService.open(entry.section.id);
			return;
		}
		this.pendingNavigationTarget = entry;
		if (this.options.settingsService.activeSectionId === entry.section.id) {
			this.navigationTree.expandTo(entry.id);
			this.navigationTree.setSelection([entry.id]);
			this.showNavigationTarget(entry.section, entry);
			this.pendingNavigationTarget = undefined;
			return;
		}
		this.options.settingsService.open(entry.section.id);
	}

	private showNavigationTarget(
		section: SettingsSectionDescriptor,
		entry: Extract<SettingsNavigationEntry, { readonly kind: 'target' }> | undefined,
	): void {
		const target = this.activePane.value?.setNavigationTarget?.(entry?.target.targetId);
		if (entry && !target) throw new RangeError(`Settings pane '${section.id}' does not expose navigation target '${entry.target.targetId}'`);
		this.activeNavigationTarget = entry;
		this.content.classList.toggle('has-navigation-target', entry !== undefined);
		if (entry) this.content.dataset.activeSettingsTarget = entry.target.targetId;
		else delete this.content.dataset.activeSettingsTarget;
		this.contentHeading.textContent = target?.title ?? this.localizedSectionLabel(section);
		this.contentDescription.textContent = target?.description ?? this.localizedSectionDescription(section);
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
		if (!this.navigationTree.focus) return;
		stopEvent(event);
		this.navigationTree.domFocus();
	}

	private filterNavigation(value: string): void {
		const query = value.trim().toLocaleLowerCase();
		this.navigationTree.setFindPattern(query);
		this.navigationScrollable.scrollTo(0, 0);
		this.navigationScrollable.layout();
	}

	private updateLocalizedChrome(): void {
		const searchLabel = this.localized('chrome.search', 'Search settings');
		this.searchInput.placeholder = searchLabel;
		this.searchInput.inputElement.setAttribute('aria-label', searchLabel);
		this.navigationEmpty.textContent = this.localized('chrome.noResults', 'No settings found.');
		this.navigationTree.rerender();
		const section = getSettingsSection(this.options.settingsService.activeSectionId);
		if (this.activeNavigationTarget) {
			this.contentHeading.textContent = this.activeNavigationTarget.target.label;
		} else {
			this.contentHeading.textContent = this.localizedSectionLabel(section);
			this.contentDescription.textContent = this.localizedSectionDescription(section);
		}
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
}

function settingsNavigationElements(): readonly ObjectTreeElement<SettingsNavigationEntry>[] {
	return SettingsSections.map(section => {
		const targets: readonly SettingsNavigationTargetDescriptor[] = 'navigationTargets' in section ? section.navigationTargets : [];
		const targetChildren = targets.map((target): ObjectTreeElement<SettingsNavigationEntry> => ({
			element: { kind: 'target', id: `${section.id}.target.${target.id}`, section, target },
		}));
		return {
			element: { kind: 'section', id: section.id, section },
			children: targetChildren,
			collapsible: targetChildren.length > 0,
			collapsed: targetChildren.length > 0,
		};
	});
}
