import "./media/sectionOverviewSettings.css";
import { h } from "../../../../base/browser/dom.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { ISettingsService } from "../../../services/preferences/common/settings.js";
import { SettingsTree } from "./settingsTree.js";
import { SettingsTreeModel, type SettingsTreeNode } from "./settingsTreeModels.js";

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
			id: "agent-behavior",
			title: "Agent behavior",
			description: "Agent execution composes rules, skills, tools, and verified Workspace context.",
			items: [
				{ id: "instructions", title: "Instructions", description: "Repository and user instruction sources constrain Agent behavior before a task starts.", status: "Managed in Rules", tone: "managed", targetSectionId: "rules", actionLabel: "Open Rules" },
				{ id: "delegation", title: "Delegation", description: "Installed Skills may define bounded subagent workflows; availability is projected into Chat.", status: "Skill-owned", tone: "managed", targetSectionId: "skills-and-subagents", actionLabel: "Manage Skills" },
				{ id: "tool-discovery", title: "Tool discovery", description: "Tool metadata is searched locally by default, with optional configured embedding ranking.", status: "Available", tone: "available", targetSectionId: "indexing", actionLabel: "Configure Indexing" },
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
				{ id: "skills", title: "Skill instructions", description: "Skills load their complete instructions only when selected or automatically activated.", status: "Managed in Skills", tone: "managed", targetSectionId: "skills-and-subagents", actionLabel: "Open Skills" },
			],
		},
	],
	"skills-and-subagents": [
		{
			id: "skills",
			title: "Skills",
			description: "Enabled Skills project callable entries directly into the unified slash-command panel.",
			items: [
				{ id: "installed", title: "Installed Skills", description: "Skill metadata is discovered from enabled packages; full instructions remain lazy until activation.", status: "Plugin-managed", tone: "managed", targetSectionId: "plugins", actionLabel: "Manage Plugins" },
				{ id: "discovery", title: "Skill discovery", description: "Install additional Skill packages from the generic Marketplace.", status: "Available", tone: "available", targetSectionId: "marketplace", actionLabel: "Browse Marketplace" },
				{ id: "subagent-workflows", title: "Subagent workflows", description: "Delegation is available only when an active Skill or Agent workflow explicitly defines bounded subagent work.", status: "Workflow-owned", tone: "managed", targetSectionId: "agents", actionLabel: "Open Agents" },
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

/** Honest capability overview used when a domain has no canonical writable settings service. */
export class SectionOverviewSettingsPane extends DisposableOwner {
	readonly element: HTMLDivElement;

	constructor(container: HTMLElement, sectionId: string, settingsService: ISettingsService) {
		super();
		const ownerDocument = container.ownerDocument;
		const groups = SectionOverviewContent[sectionId];
		if (!groups) throw new RangeError(`No Settings overview content is registered for '${sectionId}'`);
		const model = this.own(new SettingsTreeModel<OverviewItem>());
		model.setChildren(groups.map((group) => this.groupNode(sectionId, group)));
		const tree = this.own(new SettingsTree(container, {
			model,
			rootClassName: "zeta-section-overview-settings",
			groupClassName: "zeta-settings-overview-group",
			groupDescriptionClassName: "zeta-settings-overview-group-description",
			itemsClassName: "zeta-settings-overview-items",
			renderItem: (item) => this.renderItem(item.value, ownerDocument, settingsService),
		}));
		this.element = tree.element;
	}

	private renderItem(item: OverviewItem, ownerDocument: Document, settingsService: ISettingsService): HTMLElement {
		const document = ownerDocument;
		const element = h(document, "article");
		element.className = "zeta-settings-overview-item";
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
		element.append(copy);
		if (item.targetSectionId) {
			const action = this.own(new Button(element, {
				label: item.actionLabel ?? "Open settings",
				presentation: "secondary",
				size: "small",
				onClick: () => settingsService.open(item.targetSectionId),
			}));
			action.toggleClassName("zeta-settings-overview-action", true);
		}
		return element;
	}

	private groupNode(sectionId: string, group: OverviewGroup): SettingsTreeNode<OverviewItem> {
		const groupId = `${sectionId}.group.${group.id}`;
		return {
			element: { kind: "group", id: groupId, title: group.title, description: group.description },
			children: group.items.map((item): SettingsTreeNode<OverviewItem> => ({
				element: {
					kind: "item",
					id: `${groupId}.item.${item.id}`,
					title: item.title,
					description: item.description,
					keywords: [item.status, item.actionLabel ?? ""],
					value: item,
				},
			})),
		};
	}
}

export function hasSectionOverviewSettings(sectionId: string): boolean {
	return SectionOverviewContent[sectionId] !== undefined;
}
