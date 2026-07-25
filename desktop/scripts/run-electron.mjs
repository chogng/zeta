import { spawn } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const electronExecutable = require("electron");
const environment = { ...process.env };
delete environment.ELECTRON_RUN_AS_NODE;

const electron = spawn(electronExecutable, [process.cwd()], {
  env: environment,
  stdio: "inherit",
});

const stopElectron = () => {
  if (!electron.killed) electron.kill();
};

process.once("SIGINT", stopElectron);
process.once("SIGTERM", stopElectron);
electron.once("exit", (code) => {
  process.exitCode = code ?? 0;
});
