import {
  BrowserContextMenuService,
} from "../../../../platform/contextview/browser/contextMenuService.js";
import {
  type WorkbenchContextMenuServiceOptions,
  WorkbenchContextMenuService,
} from "../common/contextMenuService.js";

/** Creates the HTML context menu product service used by browser hosts. */
export function createBrowserWorkbenchContextMenuService(
  options: WorkbenchContextMenuServiceOptions,
): WorkbenchContextMenuService {
  return new WorkbenchContextMenuService(new BrowserContextMenuService(
    options.menuService,
    options.keybindingService,
    options.ownerDocument,
  ));
}
