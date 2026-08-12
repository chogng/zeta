import { type ColorScheme } from "../../../../platform/theme/common/theme.js";
import { type ExtensionThemeCatalog, type ExtensionThemeDefinition } from "../../extensions/common/extensionTheme.js";
import { type TextMateScopeTheme, type TextMateScopeThemeRule, type TextMateTokenFontStyle } from "./textMateScopeTheme.js";

/** Selects the declarative extension theme matching the active Workbench color scheme. */
export function projectExtensionTokenTheme(catalog: ExtensionThemeCatalog, colorScheme: ColorScheme, revision: number): TextMateScopeTheme {
  const theme = selectTheme(catalog.themes, colorScheme);
  return Object.freeze({ revision, rules: theme ? compileRules(theme) : Object.freeze([]) });
}

function selectTheme(themes: readonly ExtensionThemeDefinition[], colorScheme: ColorScheme): ExtensionThemeDefinition | undefined {
  const expected = colorScheme === "dark" ? "vs-dark" : colorScheme === "light" ? "vs" : colorScheme === "high-contrast-dark" ? "hc-black" : "hc-light";
  return themes.find(theme => theme.uiTheme === expected) ?? themes.find(theme => expected === "vs-dark" ? theme.uiTheme === "vs-dark" : theme.uiTheme === "vs");
}

function compileRules(theme: ExtensionThemeDefinition): readonly TextMateScopeThemeRule[] {
  const rules: TextMateScopeThemeRule[] = [];
  for (const tokenColor of theme.tokenColors) {
    const foreground = tokenColor.settings.foreground;
    const background = tokenColor.settings.background;
    const fontStyle = parseFontStyle(tokenColor.settings.fontStyle);
    if (foreground === undefined && background === undefined && fontStyle === undefined) continue;
    for (const selector of tokenColor.scopes) rules.push(Object.freeze({ selector, ...(foreground === undefined ? {} : { foreground }), ...(background === undefined ? {} : { background }), ...(fontStyle === undefined ? {} : { fontStyle }) }));
  }
  return Object.freeze(rules);
}

function parseFontStyle(value: string | undefined): readonly TextMateTokenFontStyle[] | undefined {
  if (value === undefined) return undefined;
  if (value.trim().length === 0) return Object.freeze([]);
  const styles = value.split(/\s+/u).map(style => {
    if (style !== "italic" && style !== "bold" && style !== "underline" && style !== "strikethrough") throw new TypeError(`Unsupported extension token font style '${style}'`);
    return style;
  });
  return Object.freeze([...new Set(styles)]);
}
