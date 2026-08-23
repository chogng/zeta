import type { SettingsTreeNode } from './settingsTreeModels.js';

export interface SettingsLayoutItem {
	readonly id: string;
	readonly title: string;
	readonly description: string;
	readonly keywords?: readonly string[];
}

export interface SettingsLayoutGroup<T extends SettingsLayoutItem> {
	readonly id: string;
	readonly title: string;
	readonly description: string;
	readonly settings: readonly T[];
}

/** Projects declarative groups into a validated tree with stable Settings IDs. */
export class SettingsLayout<T extends SettingsLayoutItem> {
	public readonly nodes: readonly SettingsTreeNode<T>[];

	constructor(sectionId: string, groups: readonly SettingsLayoutGroup<T>[]) {
		assertSettingsLayoutText('section ID', sectionId);
		const nodeIds = new Set<string>();
		this.nodes = groups.map(group => {
			assertSettingsLayoutText('group ID', group.id);
			assertSettingsLayoutText(`group '${group.id}' title`, group.title);
			const groupId = `${sectionId}.group.${group.id}`;
			addSettingsLayoutId(nodeIds, groupId, 'group');
			return {
				element: {
					kind: 'group',
					id: groupId,
					title: group.title,
					description: group.description,
				},
				children: group.settings.map(setting => {
					assertSettingsLayoutText('item ID', setting.id);
					assertSettingsLayoutText(`item '${setting.id}' title`, setting.title);
					addSettingsLayoutId(nodeIds, setting.id, 'item');
					return {
						element: {
							kind: 'item',
							id: setting.id,
							title: setting.title,
							description: setting.description,
							keywords: [setting.id, ...(setting.keywords ?? [])],
							value: setting,
						},
					};
				}),
			};
		});
	}
}

function assertSettingsLayoutText(label: string, value: string): void {
	if (!value.trim()) throw new TypeError(`Settings layout ${label} must not be empty`);
}

function addSettingsLayoutId(ids: Set<string>, id: string, kind: 'group' | 'item'): void {
	if (ids.has(id)) throw new TypeError(`Duplicate Settings ${kind} ID '${id}'`);
	ids.add(id);
}
