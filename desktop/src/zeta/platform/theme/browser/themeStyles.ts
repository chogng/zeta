import {
  combinedDisposable,
  type IDisposable,
  toDisposable,
} from "../../../base/common/lifecycle.js";
import {
  colorCssVariable,
  colorIdentifiers,
  type IColorTheme,
} from "../common/colorTheme.js";
import {
  type IThemeService,
} from "../common/themeService.js";
import { isDarkColorScheme } from "../common/theme.js";

interface IPreviousProperty {
  readonly value: string;
  readonly priority: string;
}

/**
 * Keeps a workbench root synchronized with its window-scoped color theme.
 *
 * Disposing the binding restores every inline property and attribute that was
 * present before the theme was applied.
 */
export function bindColorTheme(
  themeService: IThemeService,
  target: HTMLElement,
): IDisposable {
  const previousProperties = new Map<string, IPreviousProperty>();
  for (const id of colorIdentifiers) {
    const property = colorCssVariable(id);
    previousProperties.set(property, {
      value: target.style.getPropertyValue(property),
      priority: target.style.getPropertyPriority(property),
    });
  }
  previousProperties.set("color-scheme", {
    value: target.style.getPropertyValue("color-scheme"),
    priority: target.style.getPropertyPriority("color-scheme"),
  });

  const previousThemeId = target.getAttribute("data-color-theme");
  const previousColorScheme = target.getAttribute("data-color-scheme");

  const apply = (theme: IColorTheme): void => {
    for (const id of colorIdentifiers) {
      target.style.setProperty(colorCssVariable(id), theme.colors[id]);
    }
    target.style.setProperty(
      "color-scheme",
      isDarkColorScheme(theme.colorScheme) ? "dark" : "light",
    );
    target.setAttribute("data-color-theme", theme.id);
    target.setAttribute("data-color-scheme", theme.colorScheme);
  };

  apply(themeService.getColorTheme());
  const listener = themeService.onDidColorThemeChange(apply);
  const restoration = toDisposable(() => {
    for (const [property, previous] of previousProperties) {
      if (previous.value) {
        target.style.setProperty(
          property,
          previous.value,
          previous.priority,
        );
      } else {
        target.style.removeProperty(property);
      }
    }
    restoreAttribute(target, "data-color-theme", previousThemeId);
    restoreAttribute(target, "data-color-scheme", previousColorScheme);
  });

  return combinedDisposable(restoration, listener);
}

function restoreAttribute(
  target: HTMLElement,
  name: string,
  value: string | null,
): void {
  if (value === null) target.removeAttribute(name);
  else target.setAttribute(name, value);
}
