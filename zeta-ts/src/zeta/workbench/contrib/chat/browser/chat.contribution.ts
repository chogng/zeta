import "./actions/chatActions.js";
import "./actions/chatLayoutActions.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { IMenuService } from "../../../../platform/actions/common/menuService.js";
import { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import { IContextViewService } from "../../../../platform/contextview/browser/contextView.js";
import { ICommandService } from "../../../../platform/commands/common/commands.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, ViewsRegistry } from "../../../common/views.js";
import { IChatService } from "../../../services/chat/common/chatService.js";
import { IWorkbenchLayoutService } from "../../../services/layout/browser/layoutService.js";
import { ISessionsManagementService } from "../../../../sessions/services/sessions/common/sessionsManagementService.js";
import { CHAT_AGENT_SIDEBAR_VIEW_CONTAINER_ID, CHAT_AGENT_SIDEBAR_VIEW_ID, CHAT_VIEW_CONTAINER_ID, CHAT_VIEW_ID } from "../common/chat.js";
import { ChatInputEditor } from "./input/stanzaChatInputEditor.js";
import { ChatInputEditors } from "./input/chatInputEditor.js";
import { ChatAgentSidebarViewPane } from "./view/chatAgentSidebarViewPane.js";
import { ChatViewPane } from "./view/chatViewPane.js";

ChatInputEditors.registerStatic({
	id: "stanza",
	create: options => new ChatInputEditor(options),
});

/** Registers the fixed Chat view and its Workbench Agent Sidebar view. */
export function registerChatViews(registry: WorkbenchViewRegistry = ViewsRegistry): void {
	registry.registerStaticViewContainer({
		id: CHAT_VIEW_CONTAINER_ID,
		title: "Chat",
		localizationKey: { bundle: "zeta.views", key: "chat" },
		location: ViewContainerLocation.AuxiliaryBar,
		icon: lxiconsLibrary.chat,
		order: 1,
		isDefault: true,
	});
	registry.registerStaticViews(CHAT_VIEW_CONTAINER_ID, [{
		id: CHAT_VIEW_ID,
		title: "Chat",
		localizationKey: { bundle: "zeta.views", key: "chat" },
		order: 1,
		canToggleVisibility: false,
		ctorDescriptor: new SyncDescriptor(ChatViewPane, {
			serviceDependencies: [
				IChatService,
				ISessionsManagementService,
				IMenuService,
				IContextMenuService,
				IContextViewService,
				ICommandService,
				IWorkbenchLayoutService,
			],
		}),
	}]);
	registry.registerStaticViewContainer({
		id: CHAT_AGENT_SIDEBAR_VIEW_CONTAINER_ID,
		title: "Agent",
		localizationKey: { bundle: "zeta.views", key: "agent" },
		location: ViewContainerLocation.AgentSidebar,
		icon: lxiconsLibrary.agent,
		order: 1,
		isDefault: true,
	});
	registry.registerStaticViews(CHAT_AGENT_SIDEBAR_VIEW_CONTAINER_ID, [{
		id: CHAT_AGENT_SIDEBAR_VIEW_ID,
		title: "Agent Sessions",
		localizationKey: { bundle: "zeta.views", key: "agentSessions" },
		order: 1,
		canToggleVisibility: false,
		ctorDescriptor: new SyncDescriptor(ChatAgentSidebarViewPane, {
			serviceDependencies: [ISessionsManagementService],
		}),
	}]);
}
