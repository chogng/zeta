import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import {
	createServiceIdentifier,
} from "../../instantiation/common/instantiation.js";
import type { IColorTheme } from "./colorTheme.js";

/** Window-scoped access to the active workbench theme. */
export interface IThemeService {
	readonly onDidColorThemeChange: Event<IColorTheme>;

	getColorTheme(): IColorTheme;
	setColorTheme(theme: IColorTheme): void;
}

export const IThemeService =
	createServiceIdentifier<IThemeService>("themeService");

/** Owns the active color theme and notifies consumers after it changes. */
export class ThemeService extends DisposableOwner
	implements IThemeService {
	private readonly _onDidColorThemeChange = this.own(new Emitter<IColorTheme>());
	private colorTheme: IColorTheme;

	readonly onDidColorThemeChange = this._onDidColorThemeChange.event;

	constructor(initialColorTheme: IColorTheme) {
		super();
		this.colorTheme = initialColorTheme;
	}

	getColorTheme(): IColorTheme {
		return this.colorTheme;
	}

	setColorTheme(theme: IColorTheme): void {
		if (theme === this.colorTheme) return;
		this.colorTheme = theme;
		this._onDidColorThemeChange.fire(theme);
	}
}
