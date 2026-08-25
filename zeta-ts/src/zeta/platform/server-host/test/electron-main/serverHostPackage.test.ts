import { strict as assert } from "node:assert";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import { APP_SERVER_PROTOCOL_MAJOR, APP_SERVER_PROTOCOL_REVISION, APP_SERVER_SCHEMA_HASH } from "../../../../../../generated/app-server/types.js";
import { appServerDaemonExecutablePath, packagedServerHostSha256, serverHostExecutablePath } from "../../../../platform/server-host/electron-main/serverHostPackage.js";

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

test("packaged server host digest is bound to the canonical package entrypoint", () => {
	const resourcesPath = mkdtempSync(join(tmpdir(), "zeta-package-"));
	try {
		writeFileSync(join(resourcesPath, "zeta-package.json"), JSON.stringify({
			buildId: `sha256:${"b".repeat(64)}`,
			components: { serverHost: { binarySha256: "a".repeat(64) } },
			entrypoint: "bin/zeta-server.exe",
			layoutVersion: 2,
			protocol: {
				major: APP_SERVER_PROTOCOL_MAJOR,
				revision: APP_SERVER_PROTOCOL_REVISION,
				schemaHash: APP_SERVER_SCHEMA_HASH,
			},
			version: "1.2.3",
		}));
		assert.equal(packagedServerHostSha256({
			appPath: "/unused",
			expectedVersion: "1.2.3",
			isPackaged: true,
			platform: "win32",
			resourcesPath,
		}), "a".repeat(64));
	} finally {
		rmSync(resourcesPath, { recursive: true, force: true });
	}
});
