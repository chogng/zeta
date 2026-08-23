import { DisposableOwner, toDisposable } from "../../base/common/lifecycle.js";
import type { IConfigurationService } from "../../platform/configuration/common/configurationService.js";
import type { IThemeService } from "../../platform/theme/common/themeService.js";
import { WorkbenchConfiguration } from "../common/configuration.js";
import { resolveWorkbenchColorTheme, SystemColorThemePreference } from "../common/theme.js";

const DarkColorSchemeQuery = "(prefers-color-scheme: dark)";

/**
 * Resolves the persisted Workbench theme preference for one browser window.
 *
 * System-mode changes are projected immediately while explicit theme choices
 * remain stable when the operating-system preference changes.
 */
export class WorkbenchThemeController extends DisposableOwner {
	private readonly configurationService: IConfigurationService;
	private readonly themeService: IThemeService;
	private readonly systemDarkQuery: MediaQueryList;

	constructor(
		configurationService: IConfigurationService,
		themeService: IThemeService,
		ownerWindow: Window,
	) {
		super();
		this.configurationService = configurationService;
		this.themeService = themeService;
		this.systemDarkQuery = ownerWindow.matchMedia(DarkColorSchemeQuery);

		this.own(configurationService.onDidChangeConfiguration((event) => {
			if (event.affectsConfiguration(WorkbenchConfiguration.colorTheme)) {
				this.refresh();
			}
		}));
		const handleSystemSchemeChange = (): void => {
			if (
				this.configurationService.getValue(
					WorkbenchConfiguration.colorTheme,
				) === SystemColorThemePreference
			) {
				this.refresh();
			}
		};
		this.systemDarkQuery.addEventListener(
			"change",
			handleSystemSchemeChange,
		);
		this.own(toDisposable(() => {
			this.systemDarkQuery.removeEventListener(
				"change",
				handleSystemSchemeChange,
			);
		}));
		this.refresh();
	}

	/** Re-resolves the persisted ID after dynamic theme contributions change. */
	refresh(): void {
		const preference = this.configurationService.getValue(WorkbenchConfiguration.colorTheme);
		try {
			this.themeService.setColorTheme(resolveWorkbenchColorTheme(preference, this.systemDarkQuery.matches));
		} catch (error) {
			if (preference === SystemColorThemePreference) throw error;
			this.themeService.setColorTheme(resolveWorkbenchColorTheme(SystemColorThemePreference, this.systemDarkQuery.matches));
		}
	}
}
