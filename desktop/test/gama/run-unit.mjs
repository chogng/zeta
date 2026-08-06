import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

const desktopDirectory = resolve(import.meta.dirname, "../..");
const result = spawnSync(process.execPath, ["--import", "./scripts/ignore-css-imports.mjs", "--test", "--test-concurrency=1", "dist/test/src/zeta/editor/gama/**/test/**/*.test.js"], {
  cwd: desktopDirectory,
  stdio: "inherit",
});

if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
