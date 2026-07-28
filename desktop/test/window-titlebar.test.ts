import assert from "node:assert/strict";
import test from "node:test";
import {
  defaultWindowState,
} from "../src/platform/window/electron-main/window.js";
import {
  resolveBrowserWindowOptions,
} from "../src/platform/windows/electron-main/windows.js";

test("window options apply the custom titlebar host policy", () => {
  const webPreferences = {
    contextIsolation: true,
    nodeIntegration: false,
    sandbox: true,
    preload: "preload.js",
    additionalArguments: [],
  };
  const state = defaultWindowState();
  const customWindows = resolveBrowserWindowOptions({
    state,
    webPreferences,
    platform: "win32",
  });
  assert.equal(customWindows.titleBarStyle, "hidden");
  assert.deepEqual(customWindows.titleBarOverlay, {
    color: "#181818",
    symbolColor: "#d6d6d6",
    height: 35,
  });

  const customMac = resolveBrowserWindowOptions({
    state,
    webPreferences,
    platform: "darwin",
  });
  assert.equal(customMac.titleBarStyle, "hiddenInset");
  assert.equal(customMac.titleBarOverlay, true);

  const customLinux = resolveBrowserWindowOptions({
    state,
    webPreferences,
    platform: "linux",
  });
  assert.equal(customLinux.titleBarStyle, "hidden");
  assert.deepEqual(customLinux.titleBarOverlay, {
    color: "#181818",
    symbolColor: "#d6d6d6",
    height: 35,
  });
});
