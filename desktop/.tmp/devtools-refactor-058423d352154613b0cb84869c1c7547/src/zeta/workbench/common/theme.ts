import {
  type IDisposable,
  toDisposable,
} from "../../base/common/lifecycle.js";
import {
  darkColorTheme,
  type IColorTheme,
  lightColorTheme,
} from "../../platform/theme/common/colorTheme.js";

/**
 * Registry of complete color themes that can be selected by the workbench.
 *
 * Theme contributions must provide every color required by `IColorTheme`.
 */
export class WorkbenchThemeRegistry {
  readonly #themes = new Map<string, IColorTheme>();

  constructor(initialThemes: readonly IColorTheme[] = []) {
    for (const theme of initialThemes) this.#add(theme);
  }

  registerColorTheme(theme: IColorTheme): IDisposable {
    this.#add(theme);
    return toDisposable(() => {
      if (this.#themes.get(theme.id) === theme) {
        this.#themes.delete(theme.id);
      }
    });
  }

  getColorTheme(id: string): IColorTheme | undefined {
    return this.#themes.get(id);
  }

  getColorThemes(): readonly IColorTheme[] {
    return [...this.#themes.values()];
  }

  #add(theme: IColorTheme): void {
    if (!theme.id.trim()) {
      throw new TypeError("Workbench color theme ID must not be empty");
    }
    if (this.#themes.has(theme.id)) {
      throw new Error(
        `Workbench color theme is already registered: ${theme.id}`,
      );
    }
    this.#themes.set(theme.id, theme);
  }
}

/** Built-in and contributed color themes selectable by configuration. */
export const WorkbenchThemesRegistry = new WorkbenchThemeRegistry([
  darkColorTheme,
  lightColorTheme,
]);

/** Theme used before persisted configuration has been loaded. */
export const defaultWorkbenchColorTheme = darkColorTheme;

/** Resolves a validated theme identifier for a workbench window. */
export function getWorkbenchColorTheme(id: string): IColorTheme {
  const theme = WorkbenchThemesRegistry.getColorTheme(id);
  if (!theme) throw new Error(`Unknown workbench color theme: ${id}`);
  return theme;
}
