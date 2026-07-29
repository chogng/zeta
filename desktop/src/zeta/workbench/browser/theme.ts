import { DisposableOwner, toDisposable } from "../../base/common/lifecycle.js";
import type { IConfigurationService } from "../../platform/configuration/common/configuration.js";
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
  readonly #configurationService: IConfigurationService;
  readonly #themeService: IThemeService;
  readonly #systemDarkQuery: MediaQueryList;

  constructor(
    configurationService: IConfigurationService,
    themeService: IThemeService,
    ownerWindow: Window,
  ) {
    super();
    this.#configurationService = configurationService;
    this.#themeService = themeService;
    this.#systemDarkQuery = ownerWindow.matchMedia(DarkColorSchemeQuery);

    this.own(configurationService.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration(WorkbenchConfiguration.colorTheme)) {
        this.#apply();
      }
    }));
    const handleSystemSchemeChange = (): void => {
      if (
        this.#configurationService.getValue(
          WorkbenchConfiguration.colorTheme,
        ) === SystemColorThemePreference
      ) {
        this.#apply();
      }
    };
    this.#systemDarkQuery.addEventListener(
      "change",
      handleSystemSchemeChange,
    );
    this.own(toDisposable(() => {
      this.#systemDarkQuery.removeEventListener(
        "change",
        handleSystemSchemeChange,
      );
    }));
    this.#apply();
  }

  #apply(): void {
    this.#themeService.setColorTheme(resolveWorkbenchColorTheme(
      this.#configurationService.getValue(WorkbenchConfiguration.colorTheme),
      this.#systemDarkQuery.matches,
    ));
  }
}
