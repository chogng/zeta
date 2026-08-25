import { strict as assert } from "node:assert";
import { join, resolve } from "node:path";
import test from "node:test";
import { appServerDaemonExecutablePath, serverHostExecutablePath } from "../../../../platform/server-host/electron-main/serverHostPackage.js";

test("development and production resolve the same canonical package entrypoint", () => {
	assert.equal(
		appServerDaemonExecutablePath({
			appPath: "/workspace/zeta-ts",
			isPackaged: false,
			platform: "linux",
			resourcesPath: "/installed/resources",
		}),
		join(resolve("/workspace/.build/desktop/dev/zeta-package"), "bin", "zeta-app-server-daemon"),
	);
	assert.equal(
		serverHostExecutablePath({
			appPath: "/workspace/zeta-ts",
			isPackaged: false,
			platform: "linux",
			resourcesPath: "/installed/resources",
		}),
		join(resolve("/workspace/.build/desktop/dev/zeta-package"), "bin", "zeta-server"),
	);
	assert.equal(
		serverHostExecutablePath({
			appPath: "/workspace/zeta-ts",
			isPackaged: true,
			platform: "win32",
			resourcesPath: resolve("/installed/resources"),
		}),
		join(resolve("/installed/resources"), "bin", "zeta-server.exe"),
	);
	assert.equal(
		appServerDaemonExecutablePath({
			appPath: "/workspace/zeta-ts",
			isPackaged: true,
			platform: "win32",
			resourcesPath: resolve("/installed/resources"),
		}),
		join(resolve("/installed/resources"), "bin", "zeta-app-server-daemon.exe"),
	);
});
