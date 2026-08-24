export interface SettingsNavigationTargetDescriptor {
	readonly id: string;
	readonly label: string;
	readonly targetId: string;
	readonly keywords?: readonly string[];
}

export interface SettingsSectionDescriptor {
	readonly id: string;
	readonly label: string;
	readonly description: string;
	readonly navigationTargets?: readonly SettingsNavigationTargetDescriptor[];
}

/** Product-owned navigation entries and hierarchy for the Settings surface. */
export const SettingsSections = [
	{
		id: "general",
		label: "General",
		description: "Configure core application behavior and defaults.",
		navigationTargets: [
			{ id: "mode", label: "Workbench Mode", targetId: "general.group.mode" },
			{ id: "keyboard", label: "Keyboard", targetId: "general.group.keyboard" },
			{ id: "accessibility", label: "Accessibility", targetId: "general.group.accessibility" },
			{ id: "interaction", label: "Interaction", targetId: "general.group.interaction" },
		],
	},
	{
		id: "chat",
		label: "Chat",
		description: "Configure chat behavior, conversations, and presentation.",
	},
	{
		id: "user",
		label: "User",
		description: "Manage your identity, account, and user-level preferences.",
	},
	{
		id: "workspace-trust",
		label: "Workspace Trust",
		description: "Review and revoke folders that are allowed to run workspace capabilities.",
	},
	{
		id: "appearance",
		label: "Appearance",
		description: "Customize the visual appearance of Zeta.",
	},
	{
		id: "editor",
		label: "Editor",
		description: "Configure text editing, fonts, and editor behavior.",
		navigationTargets: [
			{ id: "selection", label: "Editor selection", targetId: "editor.group.selection" },
			{ id: "typography", label: "Typography", targetId: "editor.group.typography", keywords: ["font"] },
			{ id: "display", label: "Display", targetId: "editor.group.display" },
			{ id: "minimap", label: "Minimap", targetId: "editor.group.minimap" },
			{ id: "editing", label: "Editing", targetId: "editor.group.editing" },
			{
				id: "code-intelligence",
				label: "Code intelligence",
				targetId: "editor.group.code-intelligence",
				keywords: ["suggestions", "inlay hints", "code lens"],
			},
			{ id: "find-and-replace", label: "Find and replace", targetId: "editor.group.find-and-replace" },
			{ id: "workspace-search", label: "Workspace search", targetId: "editor.group.workspace-search" },
			{ id: "diff-editor", label: "Diff editor", targetId: "editor.group.diff-editor" },
			{ id: "files", label: "Files", targetId: "editor.group.files" },
		],
	},
	{
		id: "languages",
		label: "Languages",
		description: "Discover and manage Marketplace language extensions.",
	},
	{
		id: "localization",
		label: "Display Language",
		description: "Choose the language used by the Zeta interface.",
	},
	{
		id: "agents",
		label: "Agents",
		description: "Control how agents work and collaborate on tasks.",
	},
	{
		id: "models",
		label: "Models",
		description: "Choose models and configure model-specific behavior.",
	},
	{
		id: "git",
		label: "Git",
		description: "Configure source control and Git workflows.",
	},
	{
		id: "worktrees",
		label: "Worktrees",
		description: "Manage worktree creation, placement, and lifecycle.",
	},
	{
		id: "marketplace",
		label: "Marketplace",
		description: "Discover and install packages without exposing Marketplace internals to Zeta.",
	},
	{
		id: "plugins",
		label: "Plugins",
		description: "Manage installed plugins and plugin behavior.",
	},
	{
		id: "connectors",
		label: "Connectors",
		description: "Connect external accounts whose capabilities are provided by plugins.",
	},
	{
		id: "rules",
		label: "Rules",
		description: "Configure the instructions and rules agents follow.",
	},
	{
		id: "skills-and-subagents",
		label: "Skills & Subagents",
		description: "Manage reusable skills and delegated agent workflows.",
	},
	{
		id: "tools-and-mcps",
		label: "Tools & MCPs",
		description: "Configure tools and Model Context Protocol connections.",
	},
	{
		id: "hooks",
		label: "Hooks",
		description: "Configure automated actions around workflow events.",
	},
	{
		id: "browser",
		label: "Browser",
		description: "Configure browser behavior and web interactions.",
	},
	{
		id: "tabs",
		label: "Tabs",
		description: "Customize tab behavior and organization.",
	},
	{
		id: "indexing",
		label: "Indexing",
		description: "Control Agent tool discovery and workspace semantic search.",
	},
	{
		id: "experimental",
		label: "Experimental",
		description: "Try features that are still under development.",
	},
	{
		id: "documentation",
		label: "Documentation",
		description: "Configure documentation sources and related behavior.",
	},
] as const satisfies readonly SettingsSectionDescriptor[];

export function getSettingsSection(id: string): SettingsSectionDescriptor {
	return SettingsSections.find((section) => section.id === id)
		?? SettingsSections[0];
}
