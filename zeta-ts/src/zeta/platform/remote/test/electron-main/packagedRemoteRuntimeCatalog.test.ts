import { strict as assert } from "node:assert";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import { PackagedRemoteRuntimeCatalog, packagedRemoteRuntimeBundleRoot, packagedRemoteRuntimeCatalogSource } from "../../../../platform/remote/electron-main/packagedRemoteRuntimeCatalog.js";

test("packaged catalog validates and selects a release artifact", async () => {
	const root = await mkdtemp(join(tmpdir(), "zeta-remote-catalog-test-"));
	try {
		const archive = Buffer.from("canonical remote runtime");
		await writeCatalog(root, archive);

		const catalogBytes = await readFile(join(root, "catalog.json"));
		const catalog = await PackagedRemoteRuntimeCatalog.load(root, digest(catalogBytes));
		await assert.rejects(PackagedRemoteRuntimeCatalog.load(root, "a".repeat(64)), /signed product binding/);
		assert.deepEqual(catalog.artifactFor("x86_64-unknown-linux-gnu"), {
			archivePath: join(root, "artifacts", "zeta-linux.tar.gz"),
			version: "0.1.0",
			target: "x86_64-unknown-linux-gnu",
			archiveSize: archive.byteLength,
			unpackedSize: 4096,
			sha256: digest(archive),
		});
		assert.equal(catalog.artifactFor("aarch64-apple-darwin"), undefined);
	} finally {
		await rm(root, { force: true, recursive: true });
	}
});

test("signed package metadata selects exactly one local or network catalog source", async () => {
	const root = await mkdtemp(join(tmpdir(), "zeta-remote-release-binding-test-"));
	try {
		const location = { appPath: "/ignored", isPackaged: true, resourcesPath: root };
		await writeFile(join(root, "zeta-package.json"), JSON.stringify({
			remoteRuntimeCatalog: {
				url: "https://releases.example/zeta/catalog.json",
				sha256: "a".repeat(64),
				trustBinding: "signedProductPackage",
			},
		}));
		assert.deepEqual(packagedRemoteRuntimeCatalogSource(location, join(root, "cache")), {
			kind: "network",
			catalogUrl: "https://releases.example/zeta/catalog.json",
			expectedSha256: "a".repeat(64),
			cacheRoot: join(root, "cache"),
		});

		await writeFile(join(root, "zeta-package.json"), JSON.stringify({
			remoteRuntimeCatalog: {
				path: "zeta-remote-runtimes/catalog.json",
				sha256: "b".repeat(64),
				trustBinding: "signedProductPackage",
			},
		}));
		assert.deepEqual(packagedRemoteRuntimeCatalogSource(location, join(root, "cache")), {
			kind: "packaged",
			bundleRoot: join(root, "zeta-remote-runtimes"),
			expectedSha256: "b".repeat(64),
		});

		await writeFile(join(root, "zeta-package.json"), JSON.stringify({
			remoteRuntimeCatalog: {
				url: "https://user@releases.example/zeta/catalog.json",
				sha256: "a".repeat(64),
				trustBinding: "signedProductPackage",
			},
		}));
		assert.throws(() => packagedRemoteRuntimeCatalogSource(location, join(root, "cache")), /credential-free HTTPS/);
	} finally {
		await rm(root, { force: true, recursive: true });
	}
});

test("packaged catalog rejects extra fields, traversal, and duplicate targets", async () => {
	const root = await mkdtemp(join(tmpdir(), "zeta-remote-catalog-test-"));
	try {
		const archive = Buffer.from("runtime");
		await mkdir(join(root, "artifacts"));
		await writeFile(join(root, "artifacts", "zeta-linux.tar.gz"), archive);
		const record = artifactRecord(archive);
		await writeFile(join(root, "catalog.json"), JSON.stringify({ formatVersion: 1, artifacts: [{ ...record, channel: "latest" }] }));
		await assert.rejects(PackagedRemoteRuntimeCatalog.load(root), /invalid shape/);

		await writeFile(join(root, "catalog.json"), JSON.stringify({ formatVersion: 1, artifacts: [{ ...record, archive: "../escape.tar.gz" }] }));
		await assert.rejects(PackagedRemoteRuntimeCatalog.load(root), /not canonical/);

		await writeFile(join(root, "catalog.json"), JSON.stringify({ formatVersion: 1, artifacts: [record, record] }));
		await assert.rejects(PackagedRemoteRuntimeCatalog.load(root), /repeats target/);
	} finally {
		await rm(root, { force: true, recursive: true });
	}
});

test("packaged catalog rejects archive content that changed after release metadata was written", async () => {
	const root = await mkdtemp(join(tmpdir(), "zeta-remote-catalog-test-"));
	try {
		const archive = Buffer.from("runtime");
		await writeCatalog(root, archive);
		await writeFile(join(root, "artifacts", "zeta-linux.tar.gz"), Buffer.from("changed"));

		await assert.rejects(PackagedRemoteRuntimeCatalog.load(root), /SHA-256 mismatch/);
	} finally {
		await rm(root, { force: true, recursive: true });
	}
});

test("packaged catalog rejects symbolic artifact paths", { skip: process.platform === "win32" }, async () => {
	const root = await mkdtemp(join(tmpdir(), "zeta-remote-catalog-test-"));
	const outside = join(root, "outside.tar.gz");
	try {
		const archive = Buffer.from("runtime");
		await mkdir(join(root, "artifacts"));
		await writeFile(outside, archive);
		await symlink(outside, join(root, "artifacts", "zeta-linux.tar.gz"));
		await writeFile(join(root, "catalog.json"), JSON.stringify({ formatVersion: 1, artifacts: [artifactRecord(archive)] }));

		await assert.rejects(PackagedRemoteRuntimeCatalog.load(root), /symbolic path/);
	} finally {
		await rm(root, { force: true, recursive: true });
	}
});

test("bundle location follows the canonical development and packaged roots", () => {
	assert.equal(packagedRemoteRuntimeBundleRoot({ appPath: "/repo/zeta-ts", isPackaged: false, resourcesPath: "/ignored" }), resolve("/repo/.build/desktop/dev/zeta-package/zeta-remote-runtimes"));
	assert.equal(packagedRemoteRuntimeBundleRoot({ appPath: "/ignored", isPackaged: true, resourcesPath: "/Applications/Zeta.app/Contents/Resources" }), join("/Applications/Zeta.app/Contents/Resources", "zeta-remote-runtimes"));
});

async function writeCatalog(root: string, archive: Buffer): Promise<void> {
	await mkdir(join(root, "artifacts"));
	await writeFile(join(root, "artifacts", "zeta-linux.tar.gz"), archive);
	await writeFile(join(root, "catalog.json"), JSON.stringify({ formatVersion: 1, artifacts: [artifactRecord(archive)] }));
}

function artifactRecord(archive: Buffer) {
	return {
		version: "0.1.0",
		target: "x86_64-unknown-linux-gnu",
		archive: "artifacts/zeta-linux.tar.gz",
		archiveSize: archive.byteLength,
		unpackedSize: 4096,
		sha256: digest(archive),
	};
}

function digest(value: Buffer): string {
	return createHash("sha256").update(value).digest("hex");
}
