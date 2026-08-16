import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { findDesktopRoot } from "./testPaths.js";

const desktopRoot = findDesktopRoot(import.meta.dirname);
const buildRoot = join(desktopRoot, "build/vite");

test("hot reload separates the base runtime from Vite build integration", () => {
  const runtime = read("src/zeta/base/common/hotReload.ts");
  const setup = read("build/vite/setup-dev.ts");
  const plugin = read("build/vite/hotReloadPlugin.ts");
  const config = read("build/vite/vite.config.ts");

  assert.match(runtime, /registerHotReloadHandler/u);
  assert.match(runtime, /\$hotReload_applyNewExports/u);
  assert.doesNotMatch(runtime, /from\s+["']vite["']|import\.meta\.hot|workbench/u);
  assert.match(setup, /base\/common\/hotReload\.ts/u);
  assert.match(setup, /enableHotReload\(\)/u);
  assert.match(plugin, /type Plugin/u);
  assert.match(plugin, /handleHotUpdate/u);
  assert.match(config, /hotReloadPlugin/u);
});

test("Vite build ownership does not leak back into Workbench sources or scripts", () => {
  assert.equal(existsSync(join(desktopRoot, "vite.config.ts")), false);
  assert.equal(existsSync(join(desktopRoot, "src/zeta/workbench/browser/devHotReload.ts")), false);
  assert.doesNotMatch(read("src/zeta/workbench/workbench.web.main.ts"), /hotReload/u);
  assert.doesNotMatch(read("src/zeta/workbench/workbench.desktop.main.ts"), /hotReload/u);
  assert.deepEqual(readdirSync(join(desktopRoot, "scripts")).filter(file => /vite-plugin/u.test(file)), []);
  for (const file of ["hotReloadAnalysis.ts", "hotReloadPlugin.ts", "productIconsPlugin.mjs", "setup-dev.ts", "vite.config.ts", "webAppServerPlugin.mjs", "workbenchEntryPlugin.mjs"]) {
    assert.equal(existsSync(join(buildRoot, file)), true, file);
  }
});

test("Desktop Vite commands select the canonical build/vite config", () => {
  const manifest = JSON.parse(read("package.json")) as { readonly scripts: Readonly<Record<string, string>> };
  for (const name of ["build", "build:renderer", "dev", "dev:ui", "dev:renderer", "dev:web", "dev:web:full"]) {
    assert.match(manifest.scripts[name], /build\/vite\/vite\.config\.ts/u, name);
  }
});

function read(relativePath: string): string {
  return readFileSync(join(desktopRoot, relativePath), "utf8");
}
