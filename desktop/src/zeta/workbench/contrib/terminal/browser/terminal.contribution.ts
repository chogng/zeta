import { IMenuService } from "../../../../platform/actions/common/menuService.js";
import { IContextKeyService } from "../../../../platform/contextkey/common/contextkey.js";
import { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { IThemeService } from "../../../../platform/theme/common/themeService.js";
import { ViewContainerLocation, WorkbenchViewContainerId, type WorkbenchViewRegistry, ViewsRegistry } from "../../../common/views.js";
import { IWorkbenchLayoutService } from "../../../services/layout/browser/layoutService.js";
import { ITerminalService } from "../../../services/terminal/common/terminal.js";
import { TERMINAL_VIEW_ID } from "../common/terminal.js";
import { TerminalViewPane } from "./view/terminalViewPane.js";

export { TERMINAL_VIEW_ID } from "../common/terminal.js";

/** Registers the integrated terminal in the Workbench panel. */
export function registerTerminalView(registry: WorkbenchViewRegistry = ViewsRegistry): void {
  registry.registerStaticViewContainer({
    id: WorkbenchViewContainerId.Terminal,
    title: "Terminal",
    location: ViewContainerLocation.Panel,
    order: 3,
    isDefault: true,
  });
  registry.registerStaticViews(WorkbenchViewContainerId.Terminal, [{
    id: TERMINAL_VIEW_ID,
    title: "Terminal",
    order: 1,
    canToggleVisibility: false,
    ctorDescriptor: new SyncDescriptor(TerminalViewPane, {
      serviceDependencies: [ITerminalService, IThemeService, IMenuService, IContextMenuService, IContextKeyService, IWorkbenchLayoutService],
    }),
  }]);
}
