import { strict as assert } from "node:assert";
import { join, resolve } from "node:path";
import test from "node:test";
import { appServerExecutablePath } from "../../../../platform/app-server/electron-main/app-server-package.js";

test("development and production resolve the same canonical package entrypoint", () => {
  assert.equal(
    appServerExecutablePath({
      appPath: "/workspace/desktop",
      isPackaged: false,
      platform: "linux",
      resourcesPath: "/installed/resources",
    }),
    join(resolve("/workspace/desktop", ".tmp", "zeta-package"), "bin", "zeta"),
  );
  assert.equal(
    appServerExecutablePath({
      appPath: "/workspace/desktop",
      isPackaged: true,
      platform: "win32",
      resourcesPath: resolve("/installed/resources"),
    }),
    join(resolve("/installed/resources"), "bin", "zeta.exe"),
  );
});
