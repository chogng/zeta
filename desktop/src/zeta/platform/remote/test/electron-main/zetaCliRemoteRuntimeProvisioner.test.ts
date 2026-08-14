import { strict as assert } from "node:assert";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { ZetaCliRemoteRuntimeProvisioner } from "../../../../platform/remote/electron-main/zetaCliRemoteRuntimeProvisioner.js";

test("provisioner probes the Remote target and installs only its packaged artifact", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-remote-provisioner-test-"));
  try {
    const archive = Buffer.from("runtime archive");
    const archivePath = join(root, "artifacts", "zeta-linux.tar.gz");
    await mkdir(join(root, "artifacts"));
    await writeFile(archivePath, archive);
    const catalog = JSON.stringify({
      formatVersion: 1,
      artifacts: [{
        version: "0.1.0",
        target: "x86_64-unknown-linux-gnu",
        archive: "artifacts/zeta-linux.tar.gz",
        archiveSize: archive.byteLength,
        unpackedSize: 4096,
        sha256: createHash("sha256").update(archive).digest("hex"),
      }],
    });
    await writeFile(join(root, "catalog.json"), catalog);
    const invocations: string[][] = [];
    const observedSignals: Array<AbortSignal | undefined> = [];
    const progress: unknown[] = [];
    const cancellation = new AbortController();
    const provisioner = new ZetaCliRemoteRuntimeProvisioner({
      source: { kind: "packaged", bundleRoot: root, expectedSha256: createHash("sha256").update(catalog).digest("hex") },
      zetaExecutable: "/Applications/Zeta.app/Contents/Resources/bin/zeta",
      sshExecutable: "/usr/bin/ssh",
      environment: { SSH_AUTH_SOCK: "/tmp/agent.sock" },
      onProgress: event => progress.push(event),
      runCommand: async (_executable, args, _environment, observer) => {
        invocations.push([...args]);
        observedSignals.push(observer?.signal);
        if (args[1] === "probe") return { exitCode: 0, stdout: "x86_64-unknown-linux-gnu\n", stderr: "" };
        observer?.onStderrData('{"kind":"remoteRuntimeInstallProgress","phase":"complete","disposition":"reused"}\n');
        return { exitCode: 0, stdout: "/srv/zeta/remote/runtime/bin/zeta\n", stderr: "" };
      },
    });

    assert.equal(await provisioner.install("Build-Linux", { signal: cancellation.signal }), "/srv/zeta/remote/runtime/bin/zeta");
    assert.deepEqual(invocations[0], ["remote", "probe", "--host", "build-linux", "--ssh", "/usr/bin/ssh"]);
    assert.deepEqual(invocations[1], [
      "remote", "install",
      "--host", "build-linux",
      "--archive", archivePath,
      "--version", "0.1.0",
      "--target", "x86_64-unknown-linux-gnu",
      "--archive-size", String(archive.byteLength),
      "--unpacked-size", "4096",
      "--sha256", createHash("sha256").update(archive).digest("hex"),
      "--ssh", "/usr/bin/ssh",
      "--progress", "json-lines",
    ]);
    assert.deepEqual(progress, [{ phase: "complete", disposition: "reused" }]);
    assert.deepEqual(observedSignals, [cancellation.signal, cancellation.signal]);
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("network provisioner fetches the authenticated target before invoking the same SSH installer", async () => {
  const invocations: string[][] = [];
  const progress: unknown[] = [];
  const artifact = {
    archivePath: "/cache/remote-runtime.tar.gz",
    version: "0.2.0",
    target: "aarch64-unknown-linux-gnu",
    archiveSize: 2048,
    unpackedSize: 8192,
    sha256: "b".repeat(64),
  };
  const provisioner = new ZetaCliRemoteRuntimeProvisioner({
    source: {
      kind: "network",
      catalogUrl: "https://releases.example/zeta/catalog.json",
      expectedSha256: "a".repeat(64),
      cacheRoot: "/cache/zeta",
    },
    zetaExecutable: "/Applications/Zeta.app/Contents/Resources/bin/zeta",
    sshExecutable: "/usr/bin/ssh",
    environment: {},
    onProgress: event => progress.push(event),
    runCommand: async (_executable, args, _environment, observer) => {
      invocations.push([...args]);
      if (args[1] === "probe") return { exitCode: 0, stdout: `${artifact.target}\n`, stderr: "" };
      if (args[1] === "fetch-runtime") {
        observer?.onStderrData('{"kind":"remoteRuntimeDownloadProgress","phase":"downloadingArtifact","transferredBytes":1024,"totalBytes":2048}\n');
        observer?.onStderrData('{"kind":"remoteRuntimeDownloadProgress","phase":"complete","disposition":"downloaded"}\n');
        return { exitCode: 0, stdout: `${JSON.stringify(artifact)}\n`, stderr: "" };
      }
      observer?.onStderrData('{"kind":"remoteRuntimeInstallProgress","phase":"complete","disposition":"installed"}\n');
      return { exitCode: 0, stdout: "/srv/zeta/runtime/bin/zeta\n", stderr: "" };
    },
  });

  assert.equal(await provisioner.install("build-linux"), "/srv/zeta/runtime/bin/zeta");
  assert.deepEqual(invocations[1], [
    "remote", "fetch-runtime",
    "--catalog-url", "https://releases.example/zeta/catalog.json",
    "--catalog-sha256", "a".repeat(64),
    "--target", artifact.target,
    "--cache-root", "/cache/zeta",
    "--progress", "json-lines",
  ]);
  assert.deepEqual(invocations[2].slice(0, 6), ["remote", "install", "--host", "build-linux", "--archive", artifact.archivePath]);
  assert.deepEqual(progress, [
    { phase: "downloadingArtifact", transferredBytes: 1024, totalBytes: 2048 },
    { phase: "downloadComplete", disposition: "downloaded" },
    { phase: "complete", disposition: "installed" },
  ]);
});
