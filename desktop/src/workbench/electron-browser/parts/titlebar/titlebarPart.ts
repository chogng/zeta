import { isMacintosh } from "../../../../base/common/platform.js";
import type {
  INativeMenubarApi,
} from "../../../../platform/menubar/common/nativeMenubar.js";
import {
  BrowserMenubarControl,
} from "../../../browser/parts/titlebar/menubarControl.js";
import {
  BrowserTitlebarPart,
  type ITitlebarPartFactoryOptions,
  type TitlebarPartFactory,
} from "../../../browser/parts/titlebar/titlebarPart.js";
import { NativeMenubarControl } from "./nativeMenubarControl.js";
import "./titlebarpart.css";

/**
 * Desktop titlebar integration for Electron's native window controls overlay.
 *
 * Windows draws the minimize, maximize, and close controls. This part only
 * marks the draggable region and reserves space around host-provided controls.
 */
export class ElectronTitlebarPart extends BrowserTitlebarPart {
  constructor(
    options: ITitlebarPartFactoryOptions,
    nativeMenubar: INativeMenubarApi,
  ) {
    super(
      options,
      isMacintosh
        ? new NativeMenubarControl(options.menuService, nativeMenubar)
        : new BrowserMenubarControl(
          options.menuService,
          options.contextMenuService,
          options.ownerDocument,
        ),
    );
    this.element.classList.add("zeta-electron-titlebar");
  }
}

/** Creates the titlebar used by the Electron workbench. */
export function createElectronTitlebarPartFactory(
  nativeMenubar: INativeMenubarApi,
): TitlebarPartFactory {
  return (options) => new ElectronTitlebarPart(options, nativeMenubar);
}
