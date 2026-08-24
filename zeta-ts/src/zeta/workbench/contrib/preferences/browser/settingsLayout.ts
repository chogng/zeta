import type { ISetting, ISettingsEditorModel, ISettingsGroup, ISettingsSection, SettingsPresentation } from '../../../services/preferences/common/preferences.js';
import type { SettingsTreeNode } from './settingsTreeModels.js';

export interface SettingsGroupDescriptor {
	readonly id: string;
	readonly label: string;
	readonly description: string;
	readonly patterns: readonly RegExp[];
}

export interface SettingsSectionDescriptor {
	readonly id: string;
	readonly label: string;
	readonly description: string;
	readonly presentation: SettingsPresentation;
	readonly groups: readonly SettingsGroupDescriptor[];
}

/**
 * The one product-owned Settings layout.
 *
 * Configuration owners declare editable metadata in the Configuration
 * Registry. Preferences owns only where those registered settings appear.
 */
export const SettingsNavigation = [
	{
		id: 'general',
		label: 'General',
		description: 'Configure core application behavior and accessibility.',
		presentation: 'general',
		groups: [
			{
				id: 'accessibility',
				label: 'Accessibility',
				description: 'Adjust screen-reader behavior, motion, transparency, and link visibility.',
				patterns: [/^accessibility\./u, /^editor\.accessibilitySupport$/u, /^workbench\.reduce(?:Motion|Transparency)$/u],
			},
			{
				id: 'interaction',
				label: 'Interaction',
				description: 'Tune hover feedback and resize handles.',
				patterns: [/^workbench\.(?:hover|sash)\./u],
			},
		],
	},
	{
		id: 'appearance',
		label: 'Appearance',
		description: 'Customize the visual appearance of Zeta.',
		presentation: 'general',
		groups: [
			{
				id: 'theme',
				label: 'Color theme',
				description: 'Choose the colors used by the Workbench.',
				patterns: [/^workbench\.colorTheme$/u],
			},
		],
	},
	{
		id: 'editor',
		label: 'Editor',
		description: 'Configure text editing, fonts, search, and diff behavior.',
		presentation: 'editor',
		groups: [
			{
				id: 'selection',
				label: 'Editor selection',
				description: 'Choose which editor opens for new documents.',
				patterns: [/^workbench\.editor\./u],
			},
			{
				id: 'typography',
				label: 'Typography',
				description: 'Configure editor fonts and line spacing.',
				patterns: [/^editor\.(?:fontFamily|fontSize|fontLigatures|lineHeight)$/u],
			},
			{
				id: 'display',
				label: 'Display',
				description: 'Configure line wrapping, guides, highlighting, and scrolling aids.',
				patterns: [/^editor\.(?:wordWrap|lineNumbers|guides\..+|bracketPairColorization\..+|stickyScroll\..+|highlightActiveLine|unicodeHighlights)$/u],
			},
			{
				id: 'minimap',
				label: 'Minimap',
				description: 'Configure the editor document overview.',
				patterns: [/^editor\.minimap\./u],
			},
			{
				id: 'editing',
				label: 'Editing',
				description: 'Configure indentation and save-time editing behavior.',
				patterns: [/^editor\.(?:indentation|tabSize|formatOnSave)$/u],
			},
			{
				id: 'find',
				label: 'Find and replace',
				description: 'Set defaults for searches inside the active editor.',
				patterns: [/^editor\.find\./u],
			},
			{
				id: 'code-intelligence',
				label: 'Code intelligence',
				description: 'Configure completions, hints, and provider annotations.',
				patterns: [/^editor\.(?:suggest\.|inlineSuggest\.|parameterHints\.|inlayHints\.|codeLens$)/u],
			},
			{
				id: 'workspace-search',
				label: 'Workspace search',
				description: 'Set defaults for searches across workspace files.',
				patterns: [/^search\./u],
			},
			{
				id: 'diff',
				label: 'Diff editor',
				description: 'Configure how differences are displayed and navigated.',
				patterns: [/^diffEditor\./u],
			},
			{
				id: 'files',
				label: 'Files',
				description: 'Configure file editing and save behavior.',
				patterns: [/^files\./u],
			},
		],
	},
] as const satisfies readonly SettingsSectionDescriptor[];

export const SettingsSections: readonly SettingsSectionDescriptor[] = SettingsNavigation;

export function getSettingsSection(id: string): SettingsSectionDescriptor {
	return SettingsSections.find(section => section.id === id) ?? SettingsSections[0];
}

/** Projects registered configuration settings through the canonical layout. */
export function createSettingsSections(settings: readonly ISetting[]): readonly ISettingsSection[] {
	const remaining = new Map(settings.map(setting => [setting.id, setting]));
	const sections = SettingsSections.map(section => ({
		sectionId: section.id,
		groups: section.groups.map(group => {
			const matches = [...remaining.values()].filter(setting => group.patterns.some(pattern => pattern.test(setting.id)));
			for (const setting of matches) remaining.delete(setting.id);
			return {
				id: group.id,
				title: group.label,
				description: group.description,
				settings: matches.map(setting => ({ ...setting, presentation: section.presentation })),
			};
		}).filter(group => group.settings.length > 0),
	}));

	if (remaining.size > 0) {
		const ids = [...remaining.keys()].sort().join(', ');
		throw new Error(`Registered settings are missing from settingsLayout.ts: ${ids}`);
	}
	return sections;
}

/** Projects declarative groups into a validated tree with stable Settings IDs. */
export class SettingsLayout {
	public readonly nodes: readonly SettingsTreeNode<ISetting>[];

	constructor(sectionId: string, groups: readonly ISettingsGroup[]) {
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
					assertSettingsLayoutId('setting ID', setting.id);
					assertSettingsLayoutText(`setting '${setting.id}' title`, setting.title);
					addSettingsLayoutId(nodeIds, setting.id, 'setting');
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

export function settingsRootNodes(model: ISettingsEditorModel): readonly SettingsTreeNode<ISetting>[] {
	return SettingsSections.map(section => ({
		element: {
			kind: 'group',
			id: section.id,
			title: section.label,
			description: section.description,
		},
		children: new SettingsLayout(section.id, model.getSectionGroups(section.id)).nodes,
	}));
}

function assertSettingsLayoutText(label: string, value: string): void {
	if (!value.trim()) throw new TypeError(`Settings layout ${label} must not be empty`);
}

function assertSettingsLayoutId(label: string, value: string): void {
	assertSettingsLayoutText(label, value);
	if (/\p{Cc}/u.test(value)) throw new TypeError(`Settings layout ${label} must not contain control characters`);
}

function addSettingsLayoutId(ids: Set<string>, id: string, kind: 'group' | 'setting'): void {
	if (ids.has(id)) throw new TypeError(`Duplicate Settings ${kind} ID '${id}'`);
	ids.add(id);
}
