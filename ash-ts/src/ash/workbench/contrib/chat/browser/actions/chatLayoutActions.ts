import { lxiconsLibrary } from "../../../../../base/common/lxiconsLibrary.js";
import { Action2, MenuId, registerAction2 } from "../../../../../platform/actions/common/actions.js";
import { ContextKeyExpr } from "../../../../../platform/contextkey/common/contextkey.js";
import type { ServicesAccessor } from "../../../../../platform/instantiation/common/instantiation.js";
import { IPreferencesService } from "../../../../services/preferences/common/preferences.js";
import { IViewsService } from "../../../../services/views/browser/viewsService.js";
import { ChatSessionInspectorVisibleContext, CHAT_VIEW_ID, MOVE_CHAT_TO_EDITOR_COMMAND_ID, MOVE_CHAT_TO_NEW_WINDOW_COMMAND_ID, OPEN_CHAT_BROWSER_COMMAND_ID, OPEN_CHAT_SETTINGS_COMMAND_ID, TOGGLE_SESSION_INSPECTOR_COMMAND_ID } from "../../common/chat.js";
import { ChatViewPane } from "../view/chatViewPane.js";

const ChatBrowserAvailable = ContextKeyExpr.equals("chatBrowserAvailable", true);
const ChatEditorAreaAvailable = ContextKeyExpr.equals("chatEditorAreaAvailable", true);
const ChatNewWindowAvailable = ContextKeyExpr.equals("chatNewWindowAvailable", true);

registerAction2(class ToggleSessionInspectorAction extends Action2 {
	constructor() {
		super({
			id: TOGGLE_SESSION_INSPECTOR_COMMAND_ID,
			title: "Show Session Inspector",
			tooltip: "Show Session Inspector",
			icon: lxiconsLibrary.layoutSidebarRightOff,
			toggled: {
				condition: ChatSessionInspectorVisibleContext.isEqualTo(true),
				title: "Hide Session Inspector",
				tooltip: "Hide Session Inspector",
				icon: lxiconsLibrary.layoutSidebarRight,
			},
			menu: [
				{
					id: MenuId.ChatTitleLayout,
					group: "navigation",
					order: 1,
				},
			],
			f1: true,
		});
	}

	override run(accessor: ServicesAccessor): void {
		const view = accessor.get(IViewsService).openView(CHAT_VIEW_ID);
		if (view instanceof ChatViewPane) view.toggleInspector();
	}
});

registerAction2(class OpenChatBrowserAction extends Action2 {
	constructor() {
		super({
			id: OPEN_CHAT_BROWSER_COMMAND_ID,
			title: "Open Browser",
			icon: lxiconsLibrary.browserWeb,
			precondition: ChatBrowserAvailable,
			menu: {
				id: MenuId.ChatTitle,
				group: "chatActions",
				order: 1,
			},
		});
	}

	override run(): never {
		throw new Error("Open Browser is not available in this build.");
	}
});

registerAction2(class MoveChatToEditorAction extends Action2 {
	constructor() {
		super({
			id: MOVE_CHAT_TO_EDITOR_COMMAND_ID,
			title: "Move Chat to Editor Area",
			icon: lxiconsLibrary.layoutPanel,
			precondition: ChatEditorAreaAvailable,
			menu: {
				id: MenuId.ChatTitle,
				group: "chatActions",
				order: 2,
			},
		});
	}

	override run(): never {
		throw new Error("Moving Chat to the Editor Area is not available in this build.");
	}
});

registerAction2(class MoveChatToNewWindowAction extends Action2 {
	constructor() {
		super({
			id: MOVE_CHAT_TO_NEW_WINDOW_COMMAND_ID,
			title: "Move Chat to New Window",
			icon: lxiconsLibrary.linkExternal,
			precondition: ChatNewWindowAvailable,
			menu: {
				id: MenuId.ChatTitle,
				group: "chatActions",
				order: 3,
			},
		});
	}

	override run(): never {
		throw new Error("Moving Chat to a New Window is not available in this build.");
	}
});

registerAction2(class OpenChatSettingsAction extends Action2 {
	constructor() {
		super({
			id: OPEN_CHAT_SETTINGS_COMMAND_ID,
			title: "Chat Settings",
			icon: lxiconsLibrary.settings,
			menu: {
				id: MenuId.ChatTitle,
				group: "chatActions",
				order: 4,
			},
		});
	}

	override run(accessor: ServicesAccessor): Promise<void> {
		return accessor.get(IPreferencesService).openSettings();
	}
});
