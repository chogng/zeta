import {
  Menu,
  type BrowserWindow,
  type MenuItemConstructorOptions,
} from "electron/main";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { IpcRoute } from "../../app-server/electron-main/trusted-ipc-router.js";
import {
  type INativeContextMenuRequest,
  type INativeContextMenuResult,
  NATIVE_CONTEXT_MENU_CLOSE_CHANNEL,
  NATIVE_CONTEXT_MENU_POPUP_CHANNEL,
  type NativeContextMenuItem,
  validateNativeContextMenuClose,
  validateNativeContextMenuRequest,
} from "../common/nativeContextMenu.js";

/** Owns the active native context menu for one Electron window. */
export class NativeContextMenuMainService extends DisposableOwner {
  readonly #window: BrowserWindow;
  #activeMenu: Menu | undefined;
  #settle: ((result: INativeContextMenuResult) => void) | undefined;

  constructor(window: BrowserWindow) {
    super();
    this.#window = window;
    this.defer(() => {
      this.close();
      this.#finish({});
    });
  }

  popup(
    request: INativeContextMenuRequest,
  ): Promise<INativeContextMenuResult> {
    this.close();
    this.#finish({});
    if (this.#window.isDestroyed()) return Promise.resolve({});

    let selectedId: string | undefined;
    const menu = Menu.buildFromTemplate(toTemplate(
      request.items,
      (id) => {
        selectedId = id;
      },
    ));
    this.#activeMenu = menu;

    const result = new Promise<INativeContextMenuResult>((resolve) => {
      this.#settle = resolve;
    });
    try {
      menu.popup({
        window: this.#window,
        x: request.x,
        y: request.y,
        callback: () => this.#finish(
          selectedId ? { selectedId } : {},
        ),
      });
    } catch (error) {
      this.#finish({});
      throw error;
    }
    return result;
  }

  close(): void {
    const menu = this.#activeMenu;
    if (!menu) return;
    this.#activeMenu = undefined;
    if (!this.#window.isDestroyed()) menu.closePopup(this.#window);
    this.#finish({});
  }

  #finish(result: INativeContextMenuResult): void {
    const settle = this.#settle;
    if (!settle) return;
    this.#settle = undefined;
    this.#activeMenu = undefined;
    settle(result);
  }
}

export function nativeContextMenuIpcRoutes(
  service: NativeContextMenuMainService,
): readonly IpcRoute<unknown, unknown>[] {
  return [
    {
      channel: NATIVE_CONTEXT_MENU_POPUP_CHANNEL,
      validate: validateNativeContextMenuRequest,
      invoke: (request) =>
        service.popup(request as INativeContextMenuRequest),
    },
    {
      channel: NATIVE_CONTEXT_MENU_CLOSE_CHANNEL,
      validate: validateNativeContextMenuClose,
      invoke: () => {
        service.close();
      },
    },
  ];
}

function toTemplate(
  items: readonly NativeContextMenuItem[],
  select: (id: string) => void,
): MenuItemConstructorOptions[] {
  return items.map((item): MenuItemConstructorOptions => {
    switch (item.type) {
      case "separator":
        return { type: "separator" };
      case "submenu":
        return {
          type: "submenu",
          label: item.label,
          enabled: item.enabled,
          submenu: toTemplate(item.items, select),
        };
      case "action":
        return {
          type: item.checked === undefined ? "normal" : "checkbox",
          label: item.label,
          enabled: item.enabled,
          checked: item.checked,
          accelerator: item.accelerator,
          click: () => select(item.id),
        };
    }
  });
}
