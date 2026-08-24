import { h } from '../../../../base/browser/dom.js';
import { ObjectTree, type ObjectTreeCollapseStateChangeEvent, type ObjectTreeFindResult } from '../../../../base/browser/ui/tree/objectTree.js';
import type { ObjectTreeElement } from '../../../../base/browser/ui/tree/objectTreeModel.js';
import { TreeFindMatchType, TreeFindMode } from '../../../../base/browser/ui/tree/tree.js';
import { Emitter, type Event } from '../../../../base/common/event.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import type { SettingsContributionRegistry, SettingsItemContribution } from './settingsContributions.js';
import { SettingsNavigation, type SettingsSectionDescriptor, type SettingsSectionGroupDescriptor } from './settingsLayout.js';
import type { SettingsTreeNode } from './settingsTreeModels.js';

export interface SettingsTOCTarget {
	readonly id: string;
	readonly label: string;
	readonly targetId: string;
	readonly keywords?: readonly string[];
}

export type SettingsTOCEntry =
	| { readonly kind: 'group'; readonly id: string; readonly group: SettingsSectionGroupDescriptor }
	| { readonly kind: 'section'; readonly id: string; readonly section: SettingsSectionDescriptor }
	| { readonly kind: 'target'; readonly id: string; readonly section: SettingsSectionDescriptor; readonly target: SettingsTOCTarget };

export interface TOCTreeOptions {
	readonly ariaLabel: string;
	readonly groupLabel: (group: SettingsSectionGroupDescriptor) => string;
	readonly groupDescription: (group: SettingsSectionGroupDescriptor) => string;
	readonly sectionLabel: (section: SettingsSectionDescriptor) => string;
	readonly sectionDescription: (section: SettingsSectionDescriptor) => string;
}

/** Projects the product hierarchy and contributed layout groups into Settings TOC entries. */
export class TOCTreeModel {
	constructor(private readonly contributions: SettingsContributionRegistry) {}

	public get children(): readonly ObjectTreeElement<SettingsTOCEntry>[] {
		return SettingsNavigation.map(entry => {
			if ('sections' in entry) {
				return {
					element: { kind: 'group', id: `group.${entry.id}`, group: entry },
					children: entry.sections.map(section => this.sectionElement(section)),
					collapsible: true,
					collapsed: true,
				};
			}
			return this.sectionElement(entry);
		});
	}

	private sectionElement(section: SettingsSectionDescriptor): ObjectTreeElement<SettingsTOCEntry> {
		const targets = this.contributions.getSectionChildren(section.id).map(node => ({
			id: node.element.id,
			label: node.element.title,
			targetId: node.element.id,
			keywords: tocSearchKeywords(node),
		}));
		const children = targets.map((target): ObjectTreeElement<SettingsTOCEntry> => ({
			element: { kind: 'target', id: target.id, section, target },
		}));
		return {
			element: { kind: 'section', id: section.id, section },
			children,
			collapsible: children.length > 0,
			collapsed: children.length > 0,
		};
	}
}

function tocSearchKeywords(node: SettingsTreeNode<SettingsItemContribution>): readonly string[] {
	const keywords: string[] = [];
	const visit = (candidate: SettingsTreeNode<SettingsItemContribution>): void => {
		keywords.push(candidate.element.title, candidate.element.description, ...(candidate.element.keywords ?? []));
		for (const child of candidate.children ?? []) visit(child);
	};
	visit(node);
	return keywords;
}

/** Settings table of contents backed exclusively by TOCTreeModel/layout identities. */
export class TOCTree extends DisposableOwner {
	public readonly element: HTMLDivElement;
	private readonly openEmitter = this.own(new Emitter<SettingsTOCEntry>());
	private readonly tree: ObjectTree<SettingsTOCEntry>;

	public readonly onDidOpen: Event<SettingsTOCEntry> = this.openEmitter.event;
	public readonly onDidChangeCollapseState: Event<ObjectTreeCollapseStateChangeEvent<SettingsTOCEntry>>;
	public readonly onDidChangeFind: Event<ObjectTreeFindResult<SettingsTOCEntry>>;

	constructor(container: HTMLElement, private readonly model: TOCTreeModel, private readonly options: TOCTreeOptions) {
		super();
		const document = container.ownerDocument;
		this.tree = this.own(new ObjectTree(container, {
			ariaLabel: options.ariaLabel,
			scrolling: 'external',
			expandOnlyOnTwistieClick: false,
			findMatchType: TreeFindMatchType.Contiguous,
			findMode: TreeFindMode.Filter,
			getHeight: () => 26,
			indent: 12,
			keyboardNavigationLabelProvider: {
				getKeyboardNavigationLabel: entry => this.keyboardLabel(entry),
			},
			modelOptions: {
				defaultCollapseState: 'collapsed',
				identityProvider: { getId: entry => entry.id },
			},
			selectionPresentation: 'subtle',
			renderElement: entry => this.renderEntry(document, entry),
		}));
		this.element = this.tree.element;
		this.element.classList.add('zeta-settings-navigation-tree', 'zeta-settings-toc-tree');
		this.tree.setChildren(model.children);
		this.onDidChangeCollapseState = this.tree.onDidChangeCollapseState;
		this.onDidChangeFind = this.tree.onDidChangeFind;
		this.own(this.tree.onDidChangeSelection(({ elements, browserEvent }) => {
			const entry = elements[0];
			if (entry && browserEvent && entry.kind !== 'group') this.openEmitter.fire(entry);
		}));
		this.own(this.tree.onDidAccept(({ element, node }) => {
			if (node.collapsible) this.tree.toggleCollapsed(element.id);
			if (element.kind !== 'group') this.openEmitter.fire(element);
		}));
	}

	public get focus(): SettingsTOCEntry | undefined {
		return this.tree.focus;
	}

	public refresh(): void {
		const selection = this.tree.selection.map(entry => entry.id);
		const focus = this.tree.focus?.id;
		this.tree.setChildren(this.model.children);
		this.tree.setSelection(selection.filter(id => this.tree.model.has(id)));
		if (focus && this.tree.model.has(focus)) this.tree.setFocus(focus);
	}

	public expandTo(id: string): boolean {
		return this.tree.expandTo(id);
	}

	public setSelection(ids: readonly string[]): void {
		this.tree.setSelection(ids);
	}

	public setFindPattern(pattern: string): void {
		this.tree.setFindPattern(pattern);
	}

	public domFocus(): void {
		this.tree.domFocus();
	}

	public rerender(): void {
		this.tree.rerender();
	}

	private keyboardLabel(entry: SettingsTOCEntry): string {
		if (entry.kind === 'group') return `${this.options.groupLabel(entry.group)} ${this.options.groupDescription(entry.group)}`;
		if (entry.kind === 'section') return `${this.options.sectionLabel(entry.section)} ${this.options.sectionDescription(entry.section)}`;
		return [entry.target.label, ...(entry.target.keywords ?? [])].join(' ');
	}

	private renderEntry(document: Document, entry: SettingsTOCEntry): HTMLElement {
		const label = h(document, 'span');
		label.className = 'zeta-settings-navigation-label';
		if (entry.kind === 'group') label.dataset.settingsGroupId = entry.group.id;
		else if (entry.kind === 'section') label.dataset.settingsSectionId = entry.section.id;
		else label.dataset.settingsTargetId = entry.target.targetId;
		label.textContent = entry.kind === 'group'
			? this.options.groupLabel(entry.group)
			: entry.kind === 'section' ? this.options.sectionLabel(entry.section) : entry.target.label;
		return label;
	}
}
