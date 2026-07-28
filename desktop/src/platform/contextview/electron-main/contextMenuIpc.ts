import type {
  ElectronContextMenu,
} from "../../../base/parts/contextmenu/electron-main/contextmenu.js";
import {
  type INativeContextMenuRequest,
  NATIVE_CONTEXT_MENU_CLOSE_CHANNEL,
  NATIVE_CONTEXT_MENU_POPUP_CHANNEL,
  validateNativeContextMenuClose,
  validateNativeContextMenuRequest,
} from "../../../base/parts/contextmenu/common/contextmenu.js";
import type {
  IpcRoute,
} from "../../app-server/electron-main/trusted-ipc-router.js";

/** Binds the base context menu host to trusted Electron renderer IPC. */
export function nativeContextMenuIpcRoutes(
  contextMenu: ElectronContextMenu,
): readonly IpcRoute<unknown, unknown>[] {
  return [
    {
      channel: NATIVE_CONTEXT_MENU_POPUP_CHANNEL,
      validate: validateNativeContextMenuRequest,
      invoke: (request) =>
        contextMenu.popup(request as INativeContextMenuRequest),
    },
    {
      channel: NATIVE_CONTEXT_MENU_CLOSE_CHANNEL,
      validate: validateNativeContextMenuClose,
      invoke: () => {
        contextMenu.close();
      },
    },
  ];
}
