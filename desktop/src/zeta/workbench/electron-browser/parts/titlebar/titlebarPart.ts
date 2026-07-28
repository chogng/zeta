import { isMacintosh } from "../../../../base/common/platform.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type {
  INativeMenubarApi,
} from "../../../../platform/menubar/common/nativeMenubar.js";
import {
  BrowserMenubarControl,
  type IMenubarControl,
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
      new ElectronMenubarControl(options, nativeMenubar),
    );
    this.element.classList.add("zeta-electron-titlebar");
  }
}

/**
 * Keeps the compact renderer menu on every platform and mirrors it into the
 * native macOS application menu.
 */
class ElectronMenubarControl extends DisposableOwner
  implements IMenubarControl {
  readonly element: HTMLElement;

  constructor(
    options: ITitlebarPartFactoryOptions,
    nativeMenubar: INativeMenubarApi,
  ) {
    super();
    const browserMenubar = this.own(new BrowserMenubarControl(
      options.menuService,
      options.contextMenuService,
      options.ownerDocument,
    ));
    this.element = browserMenubar.element;
    if (isMacintosh) {
      this.own(new NativeMenubarControl(
        options.menuService,
        nativeMenubar,
      ));
    }
  }
}

/** Creates the titlebar used by the Electron workbench. */
export function createElectronTitlebarPartFactory(
  nativeMenubar: INativeMenubarApi,
): TitlebarPartFactory {
  return (options) => new ElectronTitlebarPart(options, nativeMenubar);
}
