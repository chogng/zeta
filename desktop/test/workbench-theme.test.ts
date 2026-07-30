import assert from "node:assert/strict";
import test from "node:test";
import { darkColorTheme } from "../src/zeta/platform/theme/common/colorTheme.js";
import { ThemeService } from "../src/zeta/platform/theme/common/themeService.js";
import { WorkbenchThemeController } from "../src/zeta/workbench/browser/theme.js";
import { WorkbenchConfiguration } from "../src/zeta/workbench/common/configuration.js";
import { WorkbenchConfigurationService } from "../src/zeta/workbench/services/configuration/browser/configurationService.js";

test("system theme follows the OS while explicit themes remain stable", async () => {
  using configuration = new WorkbenchConfigurationService();
  using themeService = new ThemeService(darkColorTheme);
  const systemTheme = new TestMediaQueryList(false);
  using controller = new WorkbenchThemeController(
    configuration,
    themeService,
    {
      matchMedia: (query: string) => {
        assert.equal(query, "(prefers-color-scheme: dark)");
        return systemTheme as unknown as MediaQueryList;
      },
    } as unknown as Window,
  );

  assert.equal(themeService.getColorTheme().id, "zeta-light");
  systemTheme.setMatches(true);
  assert.equal(themeService.getColorTheme().id, "zeta-dark");

  await configuration.updateValue(
    WorkbenchConfiguration.colorTheme,
    "zeta-light",
  );
  assert.equal(themeService.getColorTheme().id, "zeta-light");
  systemTheme.setMatches(false);
  systemTheme.setMatches(true);
  assert.equal(themeService.getColorTheme().id, "zeta-light");

  await configuration.updateValue(
    WorkbenchConfiguration.colorTheme,
    "system",
  );
  assert.equal(themeService.getColorTheme().id, "zeta-dark");
});

class TestMediaQueryList {
  private readonly listeners = new Set<() => void>();

  constructor(public matches: boolean) {}

  addEventListener(type: string, listener: () => void): void {
    if (type === "change") this.listeners.add(listener);
  }

  removeEventListener(type: string, listener: () => void): void {
    if (type === "change") this.listeners.delete(listener);
  }

  setMatches(matches: boolean): void {
    if (matches === this.matches) return;
    this.matches = matches;
    for (const listener of this.listeners) listener();
  }
}
