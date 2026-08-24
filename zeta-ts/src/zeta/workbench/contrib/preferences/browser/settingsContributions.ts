import type { Event } from '../../../../base/common/event.js';
import { Emitter } from '../../../../base/common/event.js';
import { DisposableOwner, type IDisposable } from '../../../../base/common/lifecycle.js';
import { getSettingsSection, SettingsLayout, type SettingsLayoutGroup, type SettingsLayoutItem, SettingsSections } from './settingsLayout.js';
import type { SettingsTreeNode } from './settingsTreeModels.js';

export interface SettingsItemView extends IDisposable {
	readonly element: HTMLElement;
	cancelPendingChanges?(): void;
	update?(item: SettingsItemContribution): void;
}

export interface SettingsItemContribution extends SettingsLayoutItem {
	createView(document: Document): SettingsItemView;
}

export interface SettingsSectionContribution extends IDisposable {
	readonly sectionId: string;
	readonly groups: readonly SettingsLayoutGroup<SettingsItemContribution>[];
	readonly onDidChange?: Event<void>;
	cancelPendingChanges?(): void;
}

export interface SettingsStatus {
	readonly message: string;
	readonly isError: boolean;
}

/** Owns Settings layout contributions without introducing per-category views. */
export class SettingsContributionRegistry extends DisposableOwner {
	private readonly contributions = new Map<string, SettingsSectionContribution>();
	private readonly changeEmitter = this.own(new Emitter<string>());
	private readonly statusEmitter = this.own(new Emitter<SettingsStatus>());

	public readonly onDidChangeSection = this.changeEmitter.event;
	public readonly onDidChangeStatus = this.statusEmitter.event;

	public readonly reportStatus = (message: string, isError: boolean): void => {
		this.statusEmitter.fire({ message, isError });
	};

	public register(contribution: SettingsSectionContribution): void {
		const section = getSettingsSection(contribution.sectionId);
		if (section.id !== contribution.sectionId) throw new RangeError(`Unknown Settings section '${contribution.sectionId}'`);
		if (this.contributions.has(contribution.sectionId)) throw new Error(`Settings contribution is already registered: ${contribution.sectionId}`);
		this.contributions.set(contribution.sectionId, this.own(contribution));
		if (contribution.onDidChange) {
			this.own(contribution.onDidChange(() => this.changeEmitter.fire(contribution.sectionId)));
		}
	}

	public registerLayout(sectionId: string, groups: readonly SettingsLayoutGroup<SettingsItemContribution>[]): void {
		this.register(new LayoutSettingsContribution(sectionId, groups));
	}

	public has(sectionId: string): boolean {
		return this.contributions.has(sectionId);
	}

	public get rootNodes(): readonly SettingsTreeNode<SettingsItemContribution>[] {
		return SettingsSections.map(section => this.sectionNode(section.id));
	}

	public getSectionChildren(sectionId: string): readonly SettingsTreeNode<SettingsItemContribution>[] {
		const contribution = this.contributions.get(sectionId);
		if (!contribution) throw new RangeError(`No Settings contribution is registered for '${sectionId}'`);
		return new SettingsLayout(sectionId, contribution.groups).nodes;
	}

	public cancelPendingChanges(): void {
		for (const contribution of this.contributions.values()) contribution.cancelPendingChanges?.();
	}

	private sectionNode(sectionId: string): SettingsTreeNode<SettingsItemContribution> {
		const section = getSettingsSection(sectionId);
		return {
			element: {
				kind: 'group',
				id: section.id,
				title: section.label,
				description: section.description,
			},
			children: this.getSectionChildren(section.id),
		};
	}
}

class LayoutSettingsContribution extends DisposableOwner implements SettingsSectionContribution {
	constructor(public readonly sectionId: string, public readonly groups: readonly SettingsLayoutGroup<SettingsItemContribution>[]) {
		super();
	}
}
