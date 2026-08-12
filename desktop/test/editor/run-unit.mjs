import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

const desktopDirectory = resolve(import.meta.dirname, "../..");
const result = spawnSync(process.execPath, [
  "--import",
  "./scripts/ignore-css-imports.mjs",
  "--test",
  "--test-concurrency=1",
  "dist/test/src/zeta/editor/**/test/**/*.test.js",
  "dist/test/src/zeta/workbench/contrib/academic/**/test/**/*.test.js",
  "dist/test/src/zeta/workbench/contrib/codeEditor/**/test/**/*.test.js",
  "dist/test/src/zeta/workbench/contrib/documentEditor/**/test/**/*.test.js",
  "dist/test/src/zeta/workbench/services/documentCollaboration/**/test/**/*.test.js",
  "dist/test/src/zeta/workbench/services/language/**/test/**/*.test.js",
  "dist/test/src/zeta/workbench/services/textMate/**/test/**/*.test.js",
  "dist/test/src/zeta/workbench/services/textfile/**/test/**/*.test.js",
  "dist/test/src/zeta/workbench/services/workingCopy/**/test/**/*.test.js",
], {
  cwd: desktopDirectory,
  stdio: "inherit",
});

if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
