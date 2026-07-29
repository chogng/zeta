import { LxIcon } from "../../../../base/common/lxicons.js";
import {
  Action2,
  MenuId,
  registerAction2,
} from "../../../../platform/actions/common/actions.js";
import { IMenuService } from "../../../../platform/actions/common/menuService.js";
import { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import type {
  ServicesAccessor,
} from "../../../../platform/instantiation/common/instantiation.js";
import {
  SyncDescriptor,
} from "../../../../platform/instantiation/common/instantiation.js";
import {
  IRendererApiService,
} from "../../../common/services.js";
import {
  ViewContainerLocation,
  type WorkbenchViewRegistry,
  ViewsRegistry,
} from "../../../common/views.js";
import {
  IWorkbenchSessionService,
} from "../../../services/sessions/common/sessionService.js";
import { IViewsService } from "../../../services/views/browser/viewsService.js";
import {
  CHAT_VIEW_CONTAINER_ID,
  CHAT_VIEW_ID,
  NEW_CHAT_COMMAND_ID,
  OPEN_CHAT_COMMAND_ID,
} from "../common/chat.js";
import { ChatViewPane } from "./chatViewPane.js";

/** Registers the fixed Auxiliary Chat view. */
export function registerChatViews(
  registry: WorkbenchViewRegistry = ViewsRegistry,
): void {
  registry.registerStaticViewContainer({
    id: CHAT_VIEW_CONTAINER_ID,
    title: "Chat",
    location: ViewContainerLocation.AuxiliaryBar,
    icon: LxIcon.chat,
    order: 1,
    isDefault: true,
  });
  registry.registerStaticViews(CHAT_VIEW_CONTAINER_ID, [{
    id: CHAT_VIEW_ID,
    title: "Chat",
    order: 1,
    canToggleVisibility: false,
    ctorDescriptor: new SyncDescriptor(ChatViewPane, {
      serviceDependencies: [
        IRendererApiService,
        IWorkbenchSessionService,
        IMenuService,
        IContextMenuService,
      ],
    }),
  }]);
}

registerAction2(class OpenChatAction extends Action2 {
  constructor() {
    super({
      id: OPEN_CHAT_COMMAND_ID,
      title: "Open Chat",
      icon: LxIcon.chat,
      f1: true,
    });
  }

  override run(accessor: ServicesAccessor): void {
    accessor.get(IViewsService).focusView(CHAT_VIEW_ID);
  }
});

registerAction2(class NewChatAction extends Action2 {
  constructor() {
    super({
      id: NEW_CHAT_COMMAND_ID,
      title: "New Chat",
      icon: LxIcon.add,
      f1: true,
      menu: {
        id: MenuId.ChatTitle,
        group: "navigation",
        order: 1,
      },
    });
  }

  override async run(accessor: ServicesAccessor): Promise<void> {
    const sessionService = accessor.get(IWorkbenchSessionService);
    const viewsService = accessor.get(IViewsService);
    await sessionService.startNewSession();
    viewsService.focusView(CHAT_VIEW_ID);
  }
});
