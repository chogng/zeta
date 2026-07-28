import { isMacintosh } from "../../../../base/common/platform.js";
import { BrowserContextMenuService, } from "../../../../platform/contextview/browser/contextMenuService.js";
import { NativeContextMenuService, } from "../../../../platform/contextview/electron-browser/contextMenuService.js";
import { WorkbenchContextMenuService, } from "../common/contextMenuService.js";
/**
 * Creates the Electron product service and applies the host rendering policy.
 *
 * macOS uses native menus; Windows and Linux retain the HTML implementation.
 */
export function createElectronWorkbenchContextMenuService(options, nativeApi) {
    const implementation = isMacintosh
        ? new NativeContextMenuService(nativeApi, options.menuService, options.keybindingService)
        : new BrowserContextMenuService(options.menuService, options.keybindingService, options.ownerDocument);
    return new WorkbenchContextMenuService(implementation);
}
