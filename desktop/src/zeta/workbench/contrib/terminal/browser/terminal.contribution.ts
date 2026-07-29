import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { IThemeService } from "../../../../platform/theme/common/themeService.js";
import { ViewContainerLocation, WorkbenchViewContainerId, type WorkbenchViewRegistry, ViewsRegistry } from "../../../common/views.js";
import { ITerminalService } from "../common/terminal.js";
import { TerminalViewPane } from "./terminalViewPane.js";

export const TERMINAL_VIEW_ID = "zeta.terminal";

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
      serviceDependencies: [ITerminalService, IThemeService],
    }),
  }]);
}
