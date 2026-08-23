import { bindColorTheme } from "../../../platform/theme/browser/themeStyles.js";
import { darkColorTheme } from "../../../platform/theme/common/colorTheme.js";
import { lightColorTheme } from "../../../platform/theme/common/colorTheme.js";
import { ThemeService } from "../../../platform/theme/common/themeService.js";
import { combinedDisposable } from "../../../base/common/lifecycle.js";
import { toDisposable } from "../../../base/common/lifecycle.js";
import type { IDisposable } from "../../../base/common/lifecycle.js";

const DarkColorSchemeQuery = "(prefers-color-scheme: dark)";

/** Projects a system-matched Zeta color theme into a standalone Sessions page. */
export function bindSessionsTheme(root: HTMLElement): IDisposable {
	const ownerWindow = root.ownerDocument.defaultView;
	if (!ownerWindow) throw new Error("Sessions theme requires an owner window");
	const systemColorScheme = ownerWindow.matchMedia(DarkColorSchemeQuery);
	const themeService = new ThemeService(selectTheme(systemColorScheme));
	const handleSystemColorSchemeChange = (): void => themeService.setColorTheme(selectTheme(systemColorScheme));
	systemColorScheme.addEventListener("change", handleSystemColorSchemeChange);
	return combinedDisposable(
		themeService,
		bindColorTheme(themeService, root),
		toDisposable(() => systemColorScheme.removeEventListener("change", handleSystemColorSchemeChange)),
	);
}

function selectTheme(systemColorScheme: MediaQueryList) {
	return systemColorScheme.matches ? darkColorTheme : lightColorTheme;
}
