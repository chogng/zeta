import "./actions/chatActions.js";
import "./actions/chatLayoutActions.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { IMenuService } from "../../../../platform/actions/common/menuService.js";
import { IContextKeyService } from "../../../../platform/contextkey/common/contextkey.js";
import { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import { ICommandService } from "../../../../platform/commands/common/commands.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { IRendererApiService } from "../../../common/services.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, ViewsRegistry } from "../../../common/views.js";
import { IWorkbenchLayoutService } from "../../../services/layout/browser/layoutService.js";
import { IWorkbenchSessionService } from "../../../services/sessions/common/sessionService.js";
import { IViewDescriptorService } from "../../../services/views/common/viewDescriptorService.js";
import { CHAT_VIEW_CONTAINER_ID, CHAT_VIEW_ID } from "../common/chat.js";
import { AlphaChatInputEditor } from "./input/alphaChatInputEditor.js";
import { ChatInputEditors } from "./input/chatInputEditor.js";
import { ChatViewPane } from "./view/chatViewPane.js";

ChatInputEditors.registerStatic({
  id: "alpha",
  create: options => new AlphaChatInputEditor(options),
});

/** Registers the fixed Auxiliary Chat view. */
export function registerChatViews(registry: WorkbenchViewRegistry = ViewsRegistry): void {
  registry.registerStaticViewContainer({
    id: CHAT_VIEW_CONTAINER_ID,
    title: "Chat",
    location: ViewContainerLocation.AuxiliaryBar,
    icon: lxiconsLibrary.chat,
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
        IViewDescriptorService,
        IContextKeyService,
        ICommandService,
        IWorkbenchLayoutService,
      ],
    }),
  }]);
}
