import type { SettingsTreeNode } from './settingsTreeModels.js';

export interface SettingsSectionDescriptor {
	readonly id: string;
	readonly label: string;
	readonly description: string;
}

export interface SettingsSectionGroupDescriptor {
	readonly id: string;
	readonly label: string;
	readonly description: string;
	readonly sections: readonly SettingsSectionDescriptor[];
}

export type SettingsNavigationDescriptor = SettingsSectionDescriptor | SettingsSectionGroupDescriptor;

/** Canonical product hierarchy consumed by both TOCTree and SettingsTree. */
export const SettingsNavigation = [
	{ id: 'general', label: 'General', description: 'Configure core application behavior and defaults.' },
	{ id: 'chat', label: 'Chat', description: 'Configure chat behavior, conversations, and presentation.' },
	{ id: 'user', label: 'User', description: 'Manage your identity, account, and user-level preferences.' },
	{ id: 'workspace-trust', label: 'Workspace Trust', description: 'Review and revoke folders that are allowed to run workspace capabilities.' },
	{ id: 'appearance', label: 'Appearance', description: 'Customize the visual appearance of Zeta.' },
	{ id: 'editor', label: 'Editor', description: 'Configure text editing, fonts, and editor behavior.' },
	{ id: 'languages', label: 'Languages', description: 'Discover and manage Marketplace language extensions.' },
	{ id: 'localization', label: 'Display Language', description: 'Choose the language used by the Zeta interface.' },
	{
		id: 'agents',
		label: 'Agents',
		description: 'Create agents and teams, then configure their shared capabilities.',
		sections: [
			{ id: 'agents', label: 'My Agents', description: 'Create, select, and configure reusable agents.' },
			{ id: 'teams', label: 'Teams', description: 'Compose reusable teams of agents and define how they collaborate.' },
			{ id: 'agent-defaults', label: 'Defaults', description: 'Choose the default agent and shared execution behavior.' },
			{ id: 'models', label: 'Models', description: 'Choose models and configure model-specific behavior.' },
			{ id: 'rules', label: 'Rules', description: 'Configure the instructions and rules agents follow.' },
			{ id: 'skills', label: 'Skills', description: 'Manage reusable skills that agents can activate.' },
			{ id: 'tools-and-mcps', label: 'Tools & MCPs', description: 'Configure tools and Model Context Protocol connections.' },
			{ id: 'hooks', label: 'Hooks', description: 'Configure automated actions around workflow events.' },
		],
	},
	{ id: 'git', label: 'Git', description: 'Configure source control and Git workflows.' },
	{ id: 'worktrees', label: 'Worktrees', description: 'Manage worktree creation, placement, and lifecycle.' },
	{ id: 'marketplace', label: 'Marketplace', description: 'Discover and install packages without exposing Marketplace internals to Zeta.' },
	{ id: 'plugins', label: 'Plugins', description: 'Manage installed plugins and plugin behavior.' },
	{ id: 'connectors', label: 'Connectors', description: 'Connect external accounts whose capabilities are provided by plugins.' },
	{ id: 'browser', label: 'Browser', description: 'Configure browser behavior and web interactions.' },
	{ id: 'tabs', label: 'Tabs', description: 'Customize tab behavior and organization.' },
	{ id: 'indexing', label: 'Indexing', description: 'Control Agent tool discovery and workspace semantic search.' },
	{ id: 'experimental', label: 'Experimental', description: 'Try features that are still under development.' },
	{ id: 'documentation', label: 'Documentation', description: 'Configure documentation sources and related behavior.' },
] as const satisfies readonly SettingsNavigationDescriptor[];

export const SettingsSections: readonly SettingsSectionDescriptor[] = SettingsNavigation.flatMap<SettingsSectionDescriptor>(entry =>
	'sections' in entry ? entry.sections : [entry],
);

export function getSettingsSection(id: string): SettingsSectionDescriptor {
	return SettingsSections.find(section => section.id === id) ?? SettingsSections[0];
}

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
		assertSettingsLayoutId('section ID', sectionId);
		const nodeIds = new Set<string>();
		this.nodes = groups.map(group => {
			assertSettingsLayoutId('group ID', group.id);
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
					assertSettingsLayoutId('item ID', setting.id);
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

/** Builds a collision-free item ID from domain resource identity segments. */
export function settingsResourceItemId(sectionId: string, ...segments: readonly string[]): string {
	assertSettingsLayoutId('resource section ID', sectionId);
	if (segments.length === 0) throw new TypeError('Settings resource item IDs require at least one identity segment');
	return `${sectionId}.item.${segments.map((segment, index) => {
		assertSettingsLayoutText(`resource identity segment ${index + 1}`, segment);
		return encodeURIComponent(segment).replaceAll('.', '%2E');
	}).join('.')}`;
}

function assertSettingsLayoutText(label: string, value: string): void {
	if (!value.trim()) throw new TypeError(`Settings layout ${label} must not be empty`);
}

function assertSettingsLayoutId(label: string, value: string): void {
	assertSettingsLayoutText(label, value);
	if (/\p{Cc}/u.test(value)) throw new TypeError(`Settings layout ${label} must not contain control characters`);
}

function addSettingsLayoutId(ids: Set<string>, id: string, kind: 'group' | 'item'): void {
	if (ids.has(id)) throw new TypeError(`Duplicate Settings ${kind} ID '${id}'`);
	ids.add(id);
}
