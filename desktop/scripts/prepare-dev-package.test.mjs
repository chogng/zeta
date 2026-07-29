import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { assemblePackage, hostTarget, replaceDirectoryAtomically, selectRipgrepArtifact } from "./prepare-dev-package.mjs";

test("maps supported development hosts to Rust targets", () => {
  assert.equal(hostTarget("win32", "x64"), "x86_64-pc-windows-msvc");
  assert.equal(hostTarget("darwin", "arm64"), "aarch64-apple-darwin");
  assert.equal(hostTarget("linux", "x64"), "x86_64-unknown-linux-gnu");
  assert.throws(() => hostTarget("freebsd", "x64"), /Unsupported/);
});

test("selects the target-specific locked ripgrep artifact", () => {
  const lock = {
    artifacts: {
      windows: {
        archive: "rg.zip",
        executable: "bundle/rg.exe",
        format: "zip",
        sha256: "a".repeat(64),
        size: 123,
      },
    },
    packageTargets: { "x86_64-pc-windows-msvc": "windows" },
    runtime: "ripgrep",
    schemaVersion: 1,
    source: { release: "1.0.0", repository: "https://example.invalid/repo" },
    version: "1.0.0",
  };
  const artifact = selectRipgrepArtifact(lock, "x86_64-pc-windows-msvc");
  assert.equal(artifact.executable, "bundle/rg.exe");
  assert.equal(artifact.url, "https://example.invalid/repo/releases/download/1.0.0/rg.zip");
});

test("replaces a complete development package atomically", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-dev-package-test-"));
  const output = join(root, "zeta-package");
  try {
    await mkdir(output);
    await writeFile(join(output, "generation"), "old");
    await replaceDirectoryAtomically(output, async (staging) => {
      await mkdir(staging);
      await writeFile(join(staging, "generation"), "new");
    });
    assert.equal(await readFile(join(output, "generation"), "utf8"), "new");
    assert.deepEqual(await readdir(root), ["zeta-package"]);
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("preserves the previous package when preparation fails", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-dev-package-test-"));
  const output = join(root, "zeta-package");
  try {
    await mkdir(output);
    await writeFile(join(output, "generation"), "old");
    await assert.rejects(
      replaceDirectoryAtomically(output, async (staging) => {
        await mkdir(staging);
        throw new Error("build failed");
      }),
      /build failed/,
    );
    assert.equal(await readFile(join(output, "generation"), "utf8"), "old");
    assert.deepEqual(await readdir(root), ["zeta-package"]);
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("assembles and validates the canonical Windows development layout", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-dev-package-test-"));
  const staging = join(root, "package");
  const executables = {
    commandRunner: join(root, "zeta-command-runner.exe"),
    sandboxSetup: join(root, "zeta-windows-sandbox-setup.exe"),
    zeta: join(root, "zeta.exe"),
  };
  const ripgrepExecutable = join(root, "rg.exe");
  try {
    await Promise.all([
      writeFile(executables.commandRunner, "runner"),
      writeFile(executables.sandboxSetup, "setup"),
      writeFile(executables.zeta, "zeta"),
      writeFile(ripgrepExecutable, "ripgrep"),
    ]);
    await assemblePackage(
      staging,
      "x86_64-pc-windows-msvc",
      "win32",
      executables,
      {
        archive: "rg.zip",
        archiveSha256: "a".repeat(64),
        binarySha256: "b".repeat(64),
        executable: ripgrepExecutable,
        source: "upstream-release",
        version: "1.0.0",
      },
    );
    const metadata = JSON.parse(await readFile(join(staging, "zeta-package.json"), "utf8"));
    assert.equal(metadata.entrypoint, "bin/zeta.exe");
    assert.equal(metadata.target, "x86_64-pc-windows-msvc");
    assert.equal(await readFile(join(staging, "zeta-path", "rg.exe"), "utf8"), "ripgrep");
    assert.equal(await readFile(join(staging, "zeta-resources", "zeta-command-runner.exe"), "utf8"), "runner");
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});
