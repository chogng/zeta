import { BrowserContextMenuService, } from "../../../../platform/contextview/browser/contextMenuService.js";
import { WorkbenchContextMenuService, } from "../common/contextMenuService.js";
/** Creates the HTML context menu product service used by browser hosts. */
export function createBrowserWorkbenchContextMenuService(options) {
    return new WorkbenchContextMenuService(new BrowserContextMenuService(options.menuService, options.keybindingService, options.ownerDocument));
}
