import { strict as assert } from "node:assert";
import test from "node:test";
import { ServerHostRemoteConnectionProfiles } from "../../../../platform/remote/electron-main/serverHostRemoteConnectionProfiles.js";

test("Electron Main delegates Remote profile reads, activation, and rollback to the shared Rust store", async () => {
  const invocations: Array<{ executable: string; args: readonly string[]; environment: NodeJS.ProcessEnv }> = [];
  const profiles = new ServerHostRemoteConnectionProfiles({
    serverHostExecutable: "/Applications/Zeta.app/Contents/Resources/bin/zeta-server",
    environment: { ZETA_PROFILE_ROOT: "/Users/test/Library/Application Support/Zeta/state" },
    runCommand: async (executable, args, environment) => {
      invocations.push({ executable, args, environment });
      const activeRuntime = args.includes("activate") ? args.at(-1) : "/srv/zeta/runtime/one/bin/zeta-server";
      return { exitCode: 0, stdout: JSON.stringify({ activeRuntime }), stderr: "" };
    },
  });

  assert.deepEqual(await profiles.get("Build-Linux", "/srv/project"), {
    activeRuntime: "/srv/zeta/runtime/one/bin/zeta-server",
  });
  assert.deepEqual(await profiles.activate("Build-Linux", "/srv/project", "/srv/zeta/runtime/two/bin/zeta-server"), {
    activeRuntime: "/srv/zeta/runtime/two/bin/zeta-server",
  });
  assert.deepEqual(await profiles.rollback("Build-Linux", "/srv/project", "/usr/bin/ssh"), {
    activeRuntime: "/srv/zeta/runtime/one/bin/zeta-server",
  });
  assert.deepEqual(invocations.map(invocation => invocation.args), [
    ["remote", "profile", "get", "--host", "build-linux", "--workspace", "/srv/project"],
    ["remote", "profile", "activate", "--host", "build-linux", "--workspace", "/srv/project", "--runtime", "/srv/zeta/runtime/two/bin/zeta-server"],
    ["remote", "profile", "rollback", "--host", "build-linux", "--workspace", "/srv/project", "--ssh", "/usr/bin/ssh"],
  ]);
  assert.equal(invocations[0]?.environment.ZETA_PROFILE_ROOT, "/Users/test/Library/Application Support/Zeta/state");
});

test("Remote profile adapter fails closed on command and record errors", async () => {
  const absent = new ServerHostRemoteConnectionProfiles({
    serverHostExecutable: "zeta-server",
    environment: {},
    runCommand: async () => ({ exitCode: 0, stdout: "null\n", stderr: "" }),
  });
  assert.equal(await absent.get("build-linux", "/srv/project"), undefined);

  const invalid = new ServerHostRemoteConnectionProfiles({
    serverHostExecutable: "zeta-server",
    environment: {},
    runCommand: async () => ({ exitCode: 0, stdout: '{"activeRuntime":"relative/zeta","password":"secret"}', stderr: "" }),
  });
  await assert.rejects(() => invalid.get("build-linux", "/srv/project"), /invalid record/);

  const rejected = new ServerHostRemoteConnectionProfiles({
    serverHostExecutable: "zeta-server",
    environment: {},
    runCommand: async () => ({ exitCode: 1, stdout: "", stderr: "profile busy" }),
  });
  await assert.rejects(() => rejected.get("build-linux", "/srv/project"), /profile busy/);
});
