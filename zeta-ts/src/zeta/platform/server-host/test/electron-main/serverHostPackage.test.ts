import { strict as assert } from "node:assert";
import { join, resolve } from "node:path";
import test from "node:test";
import { serverHostExecutablePath } from "../../../../platform/server-host/electron-main/serverHostPackage.js";

test("development and production resolve the same canonical package entrypoint", () => {
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
});
