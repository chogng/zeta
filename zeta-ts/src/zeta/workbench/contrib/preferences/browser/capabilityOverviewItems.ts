import "./media/capabilityOverviewItems.css";
import { h } from "../../../../base/browser/dom.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { ISettingsService } from "../../../services/preferences/common/settings.js";
import type { SettingsItemContribution, SettingsItemView, SettingsSectionContribution } from "./settingsContributions.js";

type OverviewStatusTone = "available" | "managed" | "unavailable";

interface OverviewItem {
	readonly id: string;
	readonly title: string;
	readonly description: string;
	readonly status: string;
	readonly tone: OverviewStatusTone;
	readonly targetSectionId?: string;
	readonly actionLabel?: string;
}

interface OverviewGroup {
	readonly id: string;
	readonly title: string;
	readonly description: string;
	readonly items: readonly OverviewItem[];
}

const SectionOverviewContent: Readonly<Record<string, readonly OverviewGroup[]>> = Object.freeze({
	chat: [
		{
			id: "conversations",
			title: "Conversations",
			description: "Chat state is owned by Sessions and the active conversation.",
			items: [
				{ id: "untitled", title: "Untitled conversations", description: "New chats stay window-local until the first successful send creates their durable Session.", status: "Automatic", tone: "available" },
				{ id: "tabs", title: "Conversation tabs", description: "Open Sessions retain independent Chat panes and can be reordered or closed from the Chat title area.", status: "Available", tone: "available" },
			],
		},
		{
			id: "models-and-context",
			title: "Models and context",
			description: "Model choice, connected sources, and retrieved context are managed by their owning capabilities.",
			items: [
				{ id: "conversation-model", title: "Conversation model", description: "Choose the model for each Session from the Chat surface; model credentials are supplied by installed providers.", status: "Per Session", tone: "managed", targetSectionId: "models", actionLabel: "Open Models" },
				{ id: "connected-context", title: "Connected context", description: "External accounts become available to Chat only after their plugin-provided connector is authorized.", status: "Managed in Connectors", tone: "managed", targetSectionId: "connectors", actionLabel: "Manage Connectors" },
			],
		},
	],
	user: [
		{
			id: "local-profile",
			title: "Local profile",
			description: "Zeta keeps application preferences and keybindings in the active local profile.",
			items: [
				{ id: "preferences", title: "Preferences", description: "Theme, editor, accessibility, and interaction settings persist through the configuration service.", status: "Available", tone: "available", targetSectionId: "general", actionLabel: "Open General" },
				{ id: "account-identity", title: "Account identity", description: "The current build has no application-wide cloud account service. Connected identities belong to individual connectors.", status: "Connector-owned", tone: "managed", targetSectionId: "connectors", actionLabel: "Open Connectors" },
			],
		},
		{
			id: "personalization",
			title: "Personalization",
			description: "Personal packages and presentation remain independent of one Workspace.",
			items: [
				{ id: "themes", title: "Themes", description: "Choose a built-in, extension-contributed, or editable user theme.", status: "Available", tone: "available", targetSectionId: "appearance", actionLabel: "Customize Appearance" },
				{ id: "installed-packages", title: "Installed packages", description: "Plugins expose capabilities, permissions, and optional connected accounts.", status: "Managed in Plugins", tone: "managed", targetSectionId: "plugins", actionLabel: "Manage Plugins" },
			],
		},
	],
	agents: [
		{
			id: "agent-profiles",
			title: "Agent profiles",
			description: "An Agent profile composes identity, instructions, skills, tools, and execution policy.",
			items: [
				{ id: "profiles", title: "Custom agents", description: "The current build has no canonical Agent profile service, so creation and switching are not exposed as inert controls.", status: "Not available", tone: "unavailable" },
				{ id: "delegation", title: "Subagent delegation", description: "A subagent is a bounded delegation relationship owned by an Agent or workflow, rather than a reusable Team.", status: "Workflow-owned", tone: "managed" },
				{ id: "capabilities", title: "Shared capabilities", description: "Models, rules, skills, tools, and hooks are configured independently and can later be assigned to Agent profiles.", status: "Available by category", tone: "available", targetSectionId: "models", actionLabel: "Open Models" },
			],
		},
	],
	teams: [
		{
			id: "multi-agent-teams",
			title: "Multi-agent teams",
			description: "A Team is a reusable group of Agents with explicit roles and a coordination policy.",
			items: [
				{ id: "saved-teams", title: "Saved teams", description: "No Team registry is installed in the current build, so team creation and switching are not available yet.", status: "Not available", tone: "unavailable" },
				{ id: "members", title: "Members and roles", description: "Team members remain independent Agent profiles; membership does not turn them into subagents.", status: "Team-owned", tone: "managed", targetSectionId: "agents", actionLabel: "Open My Agents" },
				{ id: "coordination", title: "Coordination policy", description: "Team mode will own routing, handoffs, shared context, concurrency, and limits when its runtime contract is added.", status: "Runtime not available", tone: "unavailable" },
			],
		},
	],
	"agent-defaults": [
		{
			id: "agent-defaults",
			title: "Agent defaults",
			description: "Defaults apply when a Session or workflow does not select a specific Agent or Team.",
			items: [
				{ id: "default-agent", title: "Default agent", description: "A default cannot be persisted until the Agent profile service provides stable Agent identities.", status: "Not available", tone: "unavailable", targetSectionId: "agents", actionLabel: "Open My Agents" },
				{ id: "default-team", title: "Default Team mode", description: "Team mode remains opt-in until a Team registry and coordination runtime are available.", status: "Not available", tone: "unavailable", targetSectionId: "teams", actionLabel: "Open Teams" },
				{ id: "default-model", title: "Model defaults", description: "Available model providers and visibility are managed independently from future Agent selection defaults.", status: "Managed in Models", tone: "managed", targetSectionId: "models", actionLabel: "Open Models" },
			],
		},
	],
	git: [
		{
			id: "source-control",
			title: "Source control",
			description: "Git operations are scoped to the active Workspace and executed through the Workspace Git service.",
			items: [
				{ id: "repository-status", title: "Repository status", description: "Changes, branch state, and upstream counts stream into the Source Control views.", status: "Available in Code", tone: "available" },
				{ id: "identity-and-remotes", title: "Git identity and remotes", description: "Zeta respects repository and user Git configuration; it does not duplicate credentials in application settings.", status: "Git-owned", tone: "managed" },
				{ id: "agent-review", title: "Agent review", description: "The Source Control area includes an Agent Review surface when the Code profile is active.", status: "Available in Code", tone: "available", targetSectionId: "agents", actionLabel: "Open Agent Settings" },
			],
		},
	],
	worktrees: [
		{
			id: "worktree-lifecycle",
			title: "Worktree lifecycle",
			description: "Worktree creation and cleanup require a dedicated repository-aware provider.",
			items: [
				{ id: "provider", title: "Worktree provider", description: "No Worktree settings or lifecycle provider is registered in the current build. Zeta will not create or remove directories from this page.", status: "Not available", tone: "unavailable" },
				{ id: "extension-path", title: "Extension path", description: "A future provider can contribute explicit create, placement, pruning, and safety policies through a plugin.", status: "Plugin extension point", tone: "managed", targetSectionId: "marketplace", actionLabel: "Browse Marketplace" },
			],
		},
	],
	rules: [
		{
			id: "instruction-sources",
			title: "Instruction sources",
			description: "Rules are durable instructions, not hidden prompt switches.",
			items: [
				{ id: "repository", title: "Repository instructions", description: "Workspace instruction files such as AGENTS.md are discovered from the repository hierarchy and applied within their scope.", status: "Workspace-owned", tone: "managed" },
				{ id: "user", title: "User instructions", description: "The current Settings service has no canonical editor for global Agent rules, so this page does not pretend to save one.", status: "Editor not available", tone: "unavailable" },
				{ id: "skills", title: "Skill instructions", description: "Skills load their complete instructions only when selected or automatically activated.", status: "Managed in Skills", tone: "managed", targetSectionId: "skills", actionLabel: "Open Skills" },
			],
		},
	],
	skills: [
		{
			id: "skills",
			title: "Skills",
			description: "Enabled Skills project callable entries directly into the unified slash-command panel.",
			items: [
				{ id: "installed", title: "Installed Skills", description: "Skill metadata is discovered from enabled packages; full instructions remain lazy until activation.", status: "Plugin-managed", tone: "managed", targetSectionId: "plugins", actionLabel: "Manage Plugins" },
				{ id: "discovery", title: "Skill discovery", description: "Install additional Skill packages from the generic Marketplace.", status: "Available", tone: "available", targetSectionId: "marketplace", actionLabel: "Browse Marketplace" },
				{ id: "agent-assignment", title: "Agent assignment", description: "Skills remain reusable capabilities that can later be assigned to Agent profiles without becoming Agents themselves.", status: "Agent-owned", tone: "managed", targetSectionId: "agents", actionLabel: "Open My Agents" },
			],
		},
	],
	"tools-and-mcps": [
		{
			id: "tool-catalog",
			title: "Tool catalog",
			description: "Tools come from the built-in runtime and enabled plugin capabilities.",
			items: [
				{ id: "tool-search", title: "Tool Search", description: "Local lexical ranking is always available; optional embedding ranking is configured separately with explicit model consent.", status: "Available", tone: "available", targetSectionId: "indexing", actionLabel: "Configure Tool Search" },
				{ id: "mcp-servers", title: "MCP servers", description: "The current Workbench has no standalone MCP configuration service. MCP capabilities must arrive through an installed plugin rather than an inert JSON field.", status: "Plugin-owned", tone: "managed", targetSectionId: "plugins", actionLabel: "Manage Plugins" },
				{ id: "external-data", title: "External data", description: "Plugin-provided tools may require an account connection before they become callable.", status: "Managed in Connectors", tone: "managed", targetSectionId: "connectors", actionLabel: "Manage Connectors" },
			],
		},
	],
	hooks: [
		{
			id: "workflow-automation",
			title: "Workflow automation",
			description: "Hooks can execute consequential actions and therefore require an explicit owner, event contract, and permission model.",
			items: [
				{ id: "runtime", title: "Hook runtime", description: "No Hook registry or event configuration service is installed in the current build, so no editable hook controls are exposed.", status: "Not available", tone: "unavailable" },
				{ id: "plugin-automation", title: "Plugin automation", description: "Plugins may expose reviewed automation capabilities and their permissions through the plugin manager.", status: "Plugin-owned", tone: "managed", targetSectionId: "plugins", actionLabel: "Manage Plugins" },
			],
		},
	],
	browser: [
		{
			id: "web-interactions",
			title: "Web interactions",
			description: "The in-app browser runs behind a host-owned navigation and automation boundary.",
			items: [
				{ id: "surfaces", title: "Browser surfaces", description: "Browser views are created by trusted Workbench capabilities and keep page navigation outside ordinary editor content.", status: "Available", tone: "available" },
				{ id: "signed-in-services", title: "Signed-in services", description: "External account access is granted through connectors instead of copying browser credentials into settings.", status: "Managed in Connectors", tone: "managed", targetSectionId: "connectors", actionLabel: "Manage Connectors" },
				{ id: "extensions", title: "Browser extensions", description: "Additional browser behavior can be supplied by reviewed plugin capabilities.", status: "Plugin-owned", tone: "managed", targetSectionId: "marketplace", actionLabel: "Browse Marketplace" },
			],
		},
	],
	tabs: [
		{
			id: "editor-tabs",
			title: "Editor tabs",
			description: "Tabs reflect the retained Editor Group state rather than creating duplicate editor models.",
			items: [
				{ id: "reorder-and-move", title: "Reorder and move", description: "Open editors can be reordered and moved between Editor Groups using the tab strip.", status: "Available", tone: "available" },
				{ id: "split-groups", title: "Split groups", description: "The editor title action can split the active group while preserving each editor input identity.", status: "Available", tone: "available" },
				{ id: "policy", title: "Tab policy", description: "Preview tabs, wrapping, and close-position policies are not configurable in the current tab component.", status: "No settings yet", tone: "unavailable" },
			],
		},
	],
	experimental: [
		{
			id: "opt-in-capabilities",
			title: "Opt-in capabilities",
			description: "Experimental behavior remains behind an explicit owner and does not silently change stable workflows.",
			items: [
				{ id: "semantic-code-search", title: "Semantic code search", description: "Configure a model endpoint, selection, Workspace authorization, and automatic context policy in Indexing.", status: "Explicit opt-in", tone: "available", targetSectionId: "indexing", actionLabel: "Open Indexing" },
				{ id: "feature-registry", title: "Experimental feature registry", description: "No generic feature-flag service is registered. New experiments must expose their own typed configuration and rollback behavior.", status: "Not available", tone: "unavailable" },
			],
		},
	],
	documentation: [
		{
			id: "document-formats",
			title: "Document formats",
			description: "Documentation support is selected by resource type and installed language capabilities.",
			items: [
				{ id: "markdown", title: "Markdown", description: "Markdown documents and previews use sanitized rendering and Workbench-owned link policy.", status: "Available", tone: "available" },
				{ id: "pdf", title: "PDF", description: "PDF resources open in the PDF editor, with annotations stored in a companion document.", status: "Available", tone: "available" },
				{ id: "academic", title: "Academic documents", description: "Structured papers, citations, references, and outlines are provided by the Academic editor profile.", status: "Academic profile", tone: "managed", targetSectionId: "general", actionLabel: "Choose Work Mode" },
				{ id: "language", title: "Language documentation", description: "Install language packages for syntax, snippets, and other language-owned contributions.", status: "Marketplace-managed", tone: "managed", targetSectionId: "languages", actionLabel: "Browse Languages" },
			],
		},
	],
});

/** Honest capability contribution used when a domain has no writable settings service. */
export class CapabilityOverviewContribution extends DisposableOwner implements SettingsSectionContribution {
	public readonly groups;

	constructor(public readonly sectionId: string, settingsService: ISettingsService) {
		super();
		const groups = SectionOverviewContent[sectionId];
		if (!groups) throw new RangeError(`No Settings overview content is registered for '${sectionId}'`);
		this.groups = groups.map(group => ({
			id: group.id,
			title: group.title,
			description: group.description,
			settings: group.items.map(item => overviewContributionItem(sectionId, group.id, item, settingsService)),
		}));
	}
}

function overviewContributionItem(sectionId: string, groupId: string, item: OverviewItem, settingsService: ISettingsService): SettingsItemContribution {
	return {
		id: `${sectionId}.group.${groupId}.item.${item.id}`,
		title: item.title,
		description: item.description,
		keywords: [item.status, item.actionLabel ?? ""],
		createView: document => new OverviewSettingsItemView(document, item, settingsService),
	};
}

class OverviewSettingsItemView extends DisposableOwner implements SettingsItemView {
	public readonly element: HTMLElement;

	constructor(document: Document, item: OverviewItem, settingsService: ISettingsService) {
		super();
		this.element = h(document, "article");
		this.element.className = "zeta-settings-overview-item";
		const copy = h(document, "div");
		copy.className = "zeta-settings-overview-copy";
		const headingRow = h(document, "div");
		headingRow.className = "zeta-settings-overview-heading";
		const title = h(document, "h5");
		title.textContent = item.title;
		const status = h(document, "span");
		status.className = `zeta-settings-overview-status ${item.tone}`;
		status.textContent = item.status;
		headingRow.append(title, status);
		const description = h(document, "p");
		description.textContent = item.description;
		copy.append(headingRow, description);
		this.element.append(copy);
		if (item.targetSectionId) {
			const action = this.own(new Button(this.element, {
				label: item.actionLabel ?? "Open settings",
				presentation: "secondary",
				size: "small",
				onClick: () => settingsService.open(item.targetSectionId),
			}));
			action.toggleClassName("zeta-settings-overview-action", true);
		}
	}
}

export function hasSectionOverviewSettings(sectionId: string): boolean {
	return SectionOverviewContent[sectionId] !== undefined;
}
