import type { ColorScheme } from '../../../platform/theme/common/theme.js';
import type { IThemeService } from '../../../platform/theme/common/themeService.js';

/** Complete standalone theme input compiled against the shared token registry. */
export interface NamedEditorThemeData {
	readonly label: string;
	readonly colorScheme: ColorScheme;
	readonly colors?: Readonly<Record<string, string>>;
}

/** Window-scoped standalone theme selection and registration. */
export interface INamedEditorThemeService extends IThemeService {
	defineNamedTheme(themeId: string, themeData: NamedEditorThemeData): void;
	setTheme(themeId: string): void;
	setAutoDetectHighContrast(autoDetectHighContrast: boolean): void;
}
