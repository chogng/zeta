import { strict as assert } from "node:assert";
import test from "node:test";
import { electronWorkspaceLaunchArguments } from "../../electron-main/electronWindowLaunch.js";

test("packaged second-instance arguments retain the Workspace target without process-only switches", () => {
  assert.deepEqual(electronWorkspaceLaunchArguments({
    arguments: ["/Applications/Zeta.app/Contents/MacOS/Zeta", "--user-data-dir", "/tmp/zeta", "--remote-ssh", "build", "--folder", "/srv/project"],
    packaging: "packaged",
    appPath: "/Applications/Zeta.app/Contents/Resources/app.asar",
  }), ["--remote-ssh", "build", "--folder", "/srv/project"]);
});

test("development second-instance arguments remove Electron and the app entry", () => {
  assert.deepEqual(electronWorkspaceLaunchArguments({
    arguments: ["/repo/node_modules/electron/dist/Electron.app/Contents/MacOS/Electron", "/repo/desktop", "/repo/desktop", "--folder", "/repo/project"],
    packaging: "development",
    appPath: "/repo/desktop",
  }), ["--folder", "/repo/project"]);
});
