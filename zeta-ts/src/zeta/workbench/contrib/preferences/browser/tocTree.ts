import { h } from '../../../../base/browser/dom.js';
import { ObjectTree, type ObjectTreeCollapseStateChangeEvent, type ObjectTreeFindResult } from '../../../../base/browser/ui/tree/objectTree.js';
import type { ObjectTreeElement } from '../../../../base/browser/ui/tree/objectTreeModel.js';
import { TreeFindMatchType, TreeFindMode } from '../../../../base/browser/ui/tree/tree.js';
import { Emitter, type Event } from '../../../../base/common/event.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import type { ISetting } from '../../../services/preferences/common/preferences.js';
import { SettingsCategories, SettingsLayout, type SettingsCategoryDescriptor, type SettingsLayoutCategory } from './settingsLayout.js';
import type { SettingsTreeNode } from './settingsTreeModels.js';

export interface SettingsTOCTarget {
	readonly id: string;
	readonly label: string;
	readonly targetId: string;
	readonly keywords?: readonly string[];
}

export type SettingsTOCEntry =
	| { readonly kind: 'category'; readonly id: string; readonly category: SettingsCategoryDescriptor }
	| { readonly kind: 'target'; readonly id: string; readonly category: SettingsCategoryDescriptor; readonly target: SettingsTOCTarget };

export interface TOCTreeOptions {
	readonly ariaLabel: string;
	readonly categoryLabel: (category: SettingsCategoryDescriptor) => string;
	readonly categoryDescription: (category: SettingsCategoryDescriptor) => string;
}

/** Projects the product hierarchy and contributed layout groups into Settings TOC entries. */
export class TOCTreeModel {
	constructor(private readonly layout: readonly SettingsLayoutCategory[]) {}

	public get children(): readonly ObjectTreeElement<SettingsTOCEntry>[] {
		return SettingsCategories.map(category => this.categoryElement(category));
	}

	private categoryElement(category: SettingsCategoryDescriptor): ObjectTreeElement<SettingsTOCEntry> {
		const groups = this.layout.find(candidate => candidate.id === category.id)?.groups ?? [];
		const targets = new SettingsLayout(category.id, groups).nodes.map(node => ({
			id: node.element.id,
			label: node.element.title,
			targetId: node.element.id,
			keywords: tocSearchKeywords(node),
		}));
		const children = targets.map((target): ObjectTreeElement<SettingsTOCEntry> => ({
			element: { kind: 'target', id: target.id, category, target },
		}));
		return {
			element: { kind: 'category', id: category.id, category },
			children,
			collapsible: children.length > 0,
			collapsed: children.length > 0,
		};
	}
}

function tocSearchKeywords(node: SettingsTreeNode<ISetting>): readonly string[] {
	const keywords: string[] = [];
	const visit = (candidate: SettingsTreeNode<ISetting>): void => {
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
			if (entry && browserEvent) this.openEmitter.fire(entry);
		}));
		this.own(this.tree.onDidAccept(({ element, node }) => {
			if (node.collapsible) this.tree.toggleCollapsed(element.id);
			this.openEmitter.fire(element);
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
		if (entry.kind === 'category') return `${this.options.categoryLabel(entry.category)} ${this.options.categoryDescription(entry.category)}`;
		return [entry.target.label, ...(entry.target.keywords ?? [])].join(' ');
	}

	private renderEntry(document: Document, entry: SettingsTOCEntry): HTMLElement {
		const label = h(document, 'span');
		label.className = 'zeta-settings-navigation-label';
		if (entry.kind === 'category') label.dataset.settingsCategoryId = entry.category.id;
		else label.dataset.settingsTargetId = entry.target.targetId;
		label.textContent = entry.kind === 'category' ? this.options.categoryLabel(entry.category) : entry.target.label;
		return label;
	}
}
