import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";
import { findDesktopRoot } from "./testPaths.js";

const desktopRoot = findDesktopRoot(import.meta.dirname);

function source(path: string): string {
  return readFileSync(resolve(desktopRoot, path), "utf8");
}

test("Desktop packages the shared backend host instead of the Zeta Code CLI", () => {
  const packageScript = source("scripts/prepare-dev-package.mjs");
  const packageManifest = source("package.json");
  const electronMain = source("src/zeta/code/electron-main/app.ts");
  const forbiddenProductCrate = ["zeta", "cli"].join("-");
  const forbiddenProductPath = ["zeta", "code", "cli"].join("/");

  for (const [name, contents] of [["package script", packageScript], ["package manifest", packageManifest], ["Electron Main", electronMain]] as const) {
    assert.equal(contents.includes(forbiddenProductCrate), false, `${name} must not reference the Zeta Code CLI crate`);
    assert.equal(contents.includes(forbiddenProductPath), false, `${name} must not reference the Zeta Code CLI source path`);
  }
  assert.match(packageScript, /cargoBuild\(target, "zeta-server-host", \["--bin", "zeta-server"\]\)/u);
  assert.match(packageManifest, /-p zeta-server-host --bin zeta-server/u);
  assert.match(electronMain, /platform\/server-host\/electron-main\/serverHostPackage\.js/u);
});
