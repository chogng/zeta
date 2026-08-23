import { app } from "electron/main";

let didBootstrap = false;

/**
 * Applies process-wide Electron settings that must exist before `ready`.
 */
export function bootstrapElectronMain(): void {
  if (didBootstrap) {
    throw new Error("Electron main bootstrap can only run once");
  }
  didBootstrap = true;

  Error.stackTraceLimit = 100;
  app.enableSandbox();
}
