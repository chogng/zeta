import { lxiconsLibrary } from "../../../../../base/common/lxiconsLibrary.js";
import { Action2, MenuId, registerAction2 } from "../../../../../platform/actions/common/actions.js";
import { ContextKeyExpr, IContextKeyService } from "../../../../../platform/contextkey/common/contextkey.js";
import type { ServicesAccessor } from "../../../../../platform/instantiation/common/instantiation.js";
import { ISettingsService } from "../../../../services/preferences/common/settings.js";
import { AgentSidebarVisibleContext, MOVE_CHAT_TO_EDITOR_COMMAND_ID, MOVE_CHAT_TO_NEW_WINDOW_COMMAND_ID, OPEN_CHAT_BROWSER_COMMAND_ID, OPEN_CHAT_SETTINGS_COMMAND_ID, TOGGLE_AGENT_SIDEBAR_COMMAND_ID } from "../../common/chat.js";

const ChatBrowserAvailable = ContextKeyExpr.equals("chatBrowserAvailable", true);
const ChatEditorAreaAvailable = ContextKeyExpr.equals("chatEditorAreaAvailable", true);
const ChatNewWindowAvailable = ContextKeyExpr.equals("chatNewWindowAvailable", true);

registerAction2(class ToggleAgentSidebarAction extends Action2 {
  constructor() {
    super({
      id: TOGGLE_AGENT_SIDEBAR_COMMAND_ID,
      title: "Show Agent Sidebar",
      tooltip: "Show Agent Sidebar",
      icon: lxiconsLibrary.layoutSidebarRightOff,
      toggled: {
        condition: AgentSidebarVisibleContext.isEqualTo(true),
        title: "Hide Agent Sidebar",
        tooltip: "Hide Agent Sidebar",
        icon: lxiconsLibrary.layoutSidebarRight,
      },
      menu: {
        id: MenuId.ChatTitleLayout,
        group: "navigation",
        order: 1,
      },
      f1: true,
    });
  }

  override run(accessor: ServicesAccessor): void {
    const contextKeys = accessor.get(IContextKeyService);
    const visible = contextKeys.getValue<boolean>(AgentSidebarVisibleContext.key) ?? AgentSidebarVisibleContext.defaultValue;
    contextKeys.setContext(AgentSidebarVisibleContext.key, !visible);
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

  override run(accessor: ServicesAccessor): void {
    accessor.get(ISettingsService).open("chat");
  }
});
