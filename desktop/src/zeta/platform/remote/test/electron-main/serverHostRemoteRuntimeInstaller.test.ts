import { strict as assert } from "node:assert";
import test from "node:test";
import { ServerHostRemoteRuntimeInstaller, remoteRuntimeArtifactFromEnvironment } from "../../../../platform/remote/electron-main/serverHostRemoteRuntimeInstaller.js";

const artifact = Object.freeze({
  archivePath: "/cache/zeta-package.tar.gz",
  version: "0.1.0",
  target: "x86_64-unknown-linux-gnu",
  archiveSize: 4096,
  unpackedSize: 16384,
  sha256: "a".repeat(64),
});

test("Electron Main delegates installation to the shared zeta remote command", async () => {
  let invocation: { executable: string; args: readonly string[]; environment: NodeJS.ProcessEnv } | undefined;
  const installer = new ServerHostRemoteRuntimeInstaller({
    serverHostExecutable: "/Applications/Zeta.app/Contents/Resources/bin/zeta-server",
    sshExecutable: "/usr/bin/ssh",
    environment: { SSH_AUTH_SOCK: "/tmp/agent.sock" },
    artifact,
    installRoot: "/srv/zeta runtime",
    runCommand: async (executable, args, environment) => {
      invocation = { executable, args, environment };
      return { exitCode: 0, stdout: "/srv/zeta runtime/runtimes/x86_64-unknown-linux-gnu/0.1.0/abc/bin/zeta-server\n", stderr: "" };
    },
  });

  const executable = await installer.install("Build-Linux");

  assert.equal(executable, "/srv/zeta runtime/runtimes/x86_64-unknown-linux-gnu/0.1.0/abc/bin/zeta-server");
  assert.deepEqual(invocation, {
    executable: "/Applications/Zeta.app/Contents/Resources/bin/zeta-server",
    args: [
      "remote", "install",
      "--host", "build-linux",
      "--archive", "/cache/zeta-package.tar.gz",
      "--version", "0.1.0",
      "--target", "x86_64-unknown-linux-gnu",
      "--archive-size", "4096",
      "--unpacked-size", "16384",
      "--sha256", "a".repeat(64),
      "--ssh", "/usr/bin/ssh",
      "--install-root", "/srv/zeta runtime",
    ],
    environment: { SSH_AUTH_SOCK: "/tmp/agent.sock" },
  });
});

test("artifact environment override is all-or-nothing and rejects unsupported targets", () => {
  assert.equal(remoteRuntimeArtifactFromEnvironment({}), undefined);
  assert.throws(
    () => remoteRuntimeArtifactFromEnvironment({ ZETA_REMOTE_RUNTIME_ARCHIVE: "/cache/runtime.tar.gz" }),
    /Incomplete Remote runtime artifact override/,
  );
  assert.throws(
    () => remoteRuntimeArtifactFromEnvironment({
      ZETA_REMOTE_RUNTIME_ARCHIVE: artifact.archivePath,
      ZETA_REMOTE_RUNTIME_VERSION: artifact.version,
      ZETA_REMOTE_RUNTIME_TARGET: "x86_64-pc-windows-msvc",
      ZETA_REMOTE_RUNTIME_ARCHIVE_SIZE: String(artifact.archiveSize),
      ZETA_REMOTE_RUNTIME_UNPACKED_SIZE: String(artifact.unpackedSize),
      ZETA_REMOTE_RUNTIME_SHA256: artifact.sha256,
    }),
    /Unsupported POSIX/,
  );
});

test("installer rejects nonzero commands and malformed receipts", async () => {
  const rejected = new ServerHostRemoteRuntimeInstaller({
    serverHostExecutable: "zeta-server",
    sshExecutable: "ssh",
    environment: {},
    artifact,
    runCommand: async () => ({ exitCode: 1, stdout: "", stderr: "digest mismatch" }),
  });
  await assert.rejects(() => rejected.install("build-linux"), /digest mismatch/);

  const malformed = new ServerHostRemoteRuntimeInstaller({
    serverHostExecutable: "zeta-server",
    sshExecutable: "ssh",
    environment: {},
    artifact,
    runCommand: async () => ({ exitCode: 0, stdout: "relative/bin/zeta-server\n", stderr: "" }),
  });
  await assert.rejects(() => malformed.install("build-linux"), /valid immutable executable path/);
});

test("installer decodes fragmented structured progress without mixing it with the result", async () => {
  const progress: unknown[] = [];
  let args: readonly string[] = [];
  const installer = new ServerHostRemoteRuntimeInstaller({
    serverHostExecutable: "zeta-server",
    sshExecutable: "ssh",
    environment: {},
    artifact,
    onProgress: event => progress.push(event),
    runCommand: async (_executable, commandArgs, _environment, observer) => {
      args = commandArgs;
      observer?.onStderrData('{"kind":"remoteRuntimeInstallProgress","phase":"validatingArtifact"}\n{"kind":"remoteRuntimeInstallProgress","phase":"upload');
      observer?.onStderrData('ing","transferredBytes":2048,"totalBytes":4096}\nplain diagnostic\n{"kind":"remoteRuntimeInstallProgress","phase":"complete","disposition":"installed"}\n');
      return { exitCode: 0, stdout: "/srv/zeta/runtime/bin/zeta-server\n", stderr: "" };
    },
  });

  await installer.install("build-linux");

  assert.deepEqual(args.slice(-2), ["--progress", "json-lines"]);
  assert.deepEqual(progress, [
    { phase: "validatingArtifact" },
    { phase: "uploading", transferredBytes: 2048, totalBytes: 4096 },
    { phase: "complete", disposition: "installed" },
  ]);
});

test("installer fails closed on malformed structured progress", async () => {
  const installer = new ServerHostRemoteRuntimeInstaller({
    serverHostExecutable: "zeta-server",
    sshExecutable: "ssh",
    environment: {},
    artifact,
    onProgress: () => {},
    runCommand: async (_executable, _args, _environment, observer) => {
      observer?.onStderrData('{"kind":"remoteRuntimeInstallProgress","phase":');
      return { exitCode: 0, stdout: "/srv/zeta/runtime/bin/zeta-server\n", stderr: "" };
    },
  });

  await assert.rejects(() => installer.install("build-linux"), /malformed progress JSON/);
});

test("installer passes request-scoped cancellation and progress to its local command", async () => {
  const cancellation = new AbortController();
  let observedSignal: AbortSignal | undefined;
  const progress: unknown[] = [];
  const installer = new ServerHostRemoteRuntimeInstaller({
    serverHostExecutable: "zeta-server",
    sshExecutable: "ssh",
    environment: {},
    artifact,
    runCommand: async (_executable, _args, _environment, observer) => {
      observedSignal = observer?.signal;
      observer?.onStderrData('{"kind":"remoteRuntimeInstallProgress","phase":"complete","disposition":"reused"}\n');
      return { exitCode: 0, stdout: "/srv/zeta/runtime/bin/zeta-server\n", stderr: "" };
    },
  });

  await installer.install("build-linux", { signal: cancellation.signal, onProgress: event => progress.push(event) });
  assert.equal(observedSignal, cancellation.signal);
  assert.deepEqual(progress, [{ phase: "complete", disposition: "reused" }]);
});
