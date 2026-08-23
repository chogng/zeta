import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { resolve } from "node:path";

const desktopRoot = resolve(import.meta.dirname, "../../zeta-ts");
const require = createRequire(resolve(desktopRoot, "package.json"));
const electronExecutable = require("electron");
const environment = { ...process.env };
delete environment.ELECTRON_RUN_AS_NODE;

const electron = spawn(
  electronExecutable,
  [desktopRoot, ...process.argv.slice(2)],
  {
    env: environment,
    stdio: "inherit",
  },
);

const stopElectron = () => {
  if (!electron.killed) electron.kill();
};

process.once("SIGINT", stopElectron);
process.once("SIGTERM", stopElectron);
electron.once("exit", (code) => {
  process.exitCode = code ?? 0;
});
