import {
  app,
  Menu,
  type BrowserWindow,
  type MenuItemConstructorOptions,
} from "electron/main";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type {
  IpcRoute,
} from "../../app-server/electron-main/trusted-ipc-router.js";
import {
  type INativeMenubarData,
  NATIVE_MENUBAR_SELECT_CHANNEL,
  NATIVE_MENUBAR_UPDATE_CHANNEL,
  type NativeMenubarItem,
  validateNativeMenubarData,
} from "../common/nativeMenubar.js";

/** Owns the macOS application menu synchronized from one workbench window. */
export class NativeMenubarMainService extends DisposableOwner {
  private readonly window: BrowserWindow;

  constructor(window: BrowserWindow) {
    super();
    this.window = window;
    this.defer(() => Menu.setApplicationMenu(null));
  }

  update(data: INativeMenubarData): void {
    const template: MenuItemConstructorOptions[] = [
      applicationMenu(),
      ...data.menus.map(({ label, items }) => ({
        label,
        submenu: toTemplate(items, (id) => this.select(data.revision, id)),
      })),
      windowMenu(),
    ];
    Menu.setApplicationMenu(Menu.buildFromTemplate(template));
  }

  private select(revision: number, id: string): void {
    if (this.window.isDestroyed()) return;
    this.window.webContents.send(NATIVE_MENUBAR_SELECT_CHANNEL, {
      revision,
      id,
    });
  }
}

export function nativeMenubarIpcRoutes(
  service: NativeMenubarMainService,
): readonly IpcRoute<unknown, unknown>[] {
  return [{
    channel: NATIVE_MENUBAR_UPDATE_CHANNEL,
    validate: validateNativeMenubarData,
    invoke: (data) => service.update(data as INativeMenubarData),
  }];
}

function toTemplate(
  items: readonly NativeMenubarItem[],
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
          click: () => select(item.id),
        };
    }
  });
}

function applicationMenu(): MenuItemConstructorOptions {
  return {
    label: app.name,
    submenu: [
      { role: "about" },
      { type: "separator" },
      { role: "services" },
      { type: "separator" },
      { role: "hide" },
      { role: "hideOthers" },
      { role: "unhide" },
      { type: "separator" },
      { role: "quit" },
    ],
  };
}

function windowMenu(): MenuItemConstructorOptions {
  return {
    role: "windowMenu",
    submenu: [
      { role: "minimize" },
      { role: "zoom" },
      { type: "separator" },
      { role: "front" },
    ],
  };
}
