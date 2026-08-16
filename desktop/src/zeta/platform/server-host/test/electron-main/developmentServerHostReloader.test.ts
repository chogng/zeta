import assert from "node:assert/strict";
import { chmod, mkdtemp, mkdir, rm, utimes, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";
import { toDisposable } from "../../../../base/common/lifecycle.js";
import type { AppServerConnectionState } from "../../../../platform/app-server/common/appServerApi.js";
import { LocalAppServerProcessLauncher } from "../../../../platform/app-server/electron-main/localAppServerProcessLauncher.js";
import { DevelopmentServerHostReloader, readDevelopmentServerHostGeneration, restartDevelopmentServerHost, selectDevelopmentServerHostExecutable } from "../../../../platform/server-host/electron-main/developmentServerHostReloader.js";

test("development Server Host generation resolves one confined built executable", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-server-host-generation-"));
  try {
    const generationDirectory = join(root, ".tmp", "dev-server-host");
    const generationFile = join(generationDirectory, "current.json");
    const executable = join(generationDirectory, "zeta-server.123.0");
    await mkdir(generationDirectory, { recursive: true });
    await writeFile(executable, "server", "utf8");
    await chmod(executable, 0o700);
    await writeFile(generationFile, `${JSON.stringify({ version: 1, executable: "zeta-server.123.0" })}\n`, "utf8");

    assert.equal(await readDevelopmentServerHostGeneration(generationFile), executable);
    await writeFile(generationFile, `${JSON.stringify({ version: 1, executable: "../zeta-server.123.0" })}\n`, "utf8");
    await assert.rejects(readDevelopmentServerHostGeneration(generationFile), /invalid/u);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("development Server Host restart selects the new executable after stopping", async () => {
  const launcher = launcherAt("/test/zeta-server.old");
  const lifecycle: string[] = [];
  const supervisor = {
    stop: async () => { lifecycle.push("stop"); },
    start: async () => { lifecycle.push(`start:${launcher.executable}`); },
  };

  await restartDevelopmentServerHost(supervisor, launcher, "/test/zeta-server.123.0");

  assert.equal(launcher.executable, "/test/zeta-server.123.0");
  assert.deepEqual(lifecycle, ["stop", "start:/test/zeta-server.123.0"]);
});

test("development Server Host startup ignores a generation older than the assembled package", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-server-host-selection-"));
  try {
    const packaged = join(root, "packaged-zeta-server");
    const development = join(root, "development-zeta-server");
    await writeFile(packaged, "packaged", "utf8");
    await writeFile(development, "development", "utf8");
    await utimes(development, new Date(1_000), new Date(1_000));
    await utimes(packaged, new Date(2_000), new Date(2_000));
    assert.equal(selectDevelopmentServerHostExecutable(packaged, development), packaged);
    await utimes(development, new Date(3_000), new Date(3_000));
    assert.equal(selectDevelopmentServerHostExecutable(packaged, development), development);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("development Server Host restart restores the previous generation after failure", async () => {
  const launcher = launcherAt("/test/zeta-server.old");
  const lifecycle: string[] = [];
  let starts = 0;
  const supervisor = {
    stop: async () => { lifecycle.push("stop"); },
    start: async () => {
      lifecycle.push(`start:${launcher.executable}`);
      if (starts++ === 0) throw new Error("new generation failed");
    },
  };

  await assert.rejects(restartDevelopmentServerHost(supervisor, launcher, "/test/zeta-server.123.0"), /new generation failed/u);

  assert.equal(launcher.executable, "/test/zeta-server.old");
  assert.deepEqual(lifecycle, [
    "stop",
    "start:/test/zeta-server.123.0",
    "stop",
    "start:/test/zeta-server.old",
  ]);
});

test("development Server Host queues a generation until initial startup is stable", async () => {
  const launcher = launcherAt("/test/zeta-server.old");
  const listeners = new Set<(state: AppServerConnectionState) => void>();
  let state: AppServerConnectionState = "initializing";
  const supervisor = {
    get state() { return state; },
    onStateChange(listener: (state: AppServerConnectionState) => void) {
      listeners.add(listener);
      return toDisposable(() => listeners.delete(listener));
    },
    stop: async () => { state = "stopped"; },
    start: async () => { state = "ready"; },
  };
  const reloader = new DevelopmentServerHostReloader({
    generationFile: "/test/current.json",
    launcher,
    supervisor,
    watchGeneration: () => toDisposable(() => {}),
    readGeneration: async () => "/test/zeta-server.123.0",
    log: () => {},
  });

  await reloader.reloadNow();
  assert.equal(launcher.executable, "/test/zeta-server.old");
  state = "stopped";
  for (const listener of listeners) listener(state);
  await new Promise<void>(resolve => setImmediate(resolve));
  assert.equal(launcher.executable, "/test/zeta-server.123.0");
  reloader.dispose();
});

function launcherAt(executable: string): LocalAppServerProcessLauncher {
  return new LocalAppServerProcessLauncher({
    executable,
    args: ["app-server", "connect"],
    environment: {},
    fileExists: () => true,
  });
}
