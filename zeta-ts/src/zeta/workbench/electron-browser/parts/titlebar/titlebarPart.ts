import { isMacintosh } from "../../../../base/common/platform.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
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
		container: HTMLElement,
		options: ITitlebarPartFactoryOptions,
		nativeMenubar: INativeMenubarApi,
	) {
		super(
			container,
			options,
			new ElectronMenubarControl(container, options, nativeMenubar),
		);
		this.domNode.classList.add("zeta-electron-titlebar");
	}
}

/**
 * Keeps the compact renderer menu on every platform and mirrors it into the
 * native macOS application menu.
 */
class ElectronMenubarControl extends Disposable
	implements IMenubarControl {
	readonly domNode: HTMLElement;

	constructor(
		container: HTMLElement,
		options: ITitlebarPartFactoryOptions,
		nativeMenubar: INativeMenubarApi,
	) {
		super();
		const browserMenubar = this._register(new BrowserMenubarControl(
			container,
			options.menuService,
			options.contextMenuService,
			options.localizationService,
		));
		this.domNode = browserMenubar.domNode;
		if (isMacintosh) {
			this._register(new NativeMenubarControl(
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
	return (container, options) => new ElectronTitlebarPart(container, options, nativeMenubar);
}
