import { isMacintosh } from "../../../../base/common/platform.js";
import type {
	INativeContextMenuApi,
} from "../../../../base/parts/contextmenu/common/contextmenu.js";
import {
	BrowserContextMenuService,
} from "../../../../platform/contextview/browser/contextMenuService.js";
import {
	NativeContextMenuService,
} from "../../../../platform/contextview/electron-browser/contextMenuService.js";
import {
	type WorkbenchContextMenuServiceOptions,
	WorkbenchContextMenuService,
} from "../browser/workbenchContextMenuService.js";

/**
 * Creates the Electron product service and applies the host rendering policy.
 *
 * macOS uses native menus; Windows and Linux retain the HTML implementation.
 */
export function createElectronWorkbenchContextMenuService(
	options: WorkbenchContextMenuServiceOptions,
	nativeApi: INativeContextMenuApi,
): WorkbenchContextMenuService {
	const implementation = isMacintosh
		? new NativeContextMenuService(
			nativeApi,
			options.menuService,
			options.keybindingService,
		)
		: new BrowserContextMenuService(
			options.menuService,
			options.keybindingService,
			options.contextViewService,
		);
	return new WorkbenchContextMenuService(implementation);
}
