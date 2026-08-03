import assert from "node:assert/strict";
import test from "node:test";
import { buildAppServerEnvironment, isAllowedAppServerEnvironmentKey } from "../../../../platform/app-server/common/appServerEnvironment.js";

test("App Server environment keeps safe POSIX session variables and excludes credentials", () => {
  const environment = buildAppServerEnvironment({
    HOME: "/home/zeta",
    LANG: "en_US.UTF-8",
    LC_ALL: "C.UTF-8",
    PATH: "/usr/bin",
    XDG_CONFIG_HOME: "/home/zeta/.config",
    OPENAI_API_KEY: "secret",
  }, "posix", {
    ZETA_PROFILE_ROOT: "/state",
    ZETA_WORKSPACE_ROOT: "/workspace",
  });

  assert.deepEqual(environment, {
    HOME: "/home/zeta",
    LANG: "en_US.UTF-8",
    PATH: "/usr/bin",
    XDG_CONFIG_HOME: "/home/zeta/.config",
    LC_ALL: "C.UTF-8",
    ZETA_PROFILE_ROOT: "/state",
    ZETA_WORKSPACE_ROOT: "/workspace",
  });
  assert.equal(isAllowedAppServerEnvironmentKey("OPENAI_API_KEY"), false);
});

test("App Server environment canonicalizes Windows keys case-insensitively", () => {
  const environment = buildAppServerEnvironment({
    Path: "C:\\Windows\\System32",
    SystemRoot: "C:\\Windows",
    UserProfile: "C:\\Users\\zeta",
    AWS_SECRET_ACCESS_KEY: "secret",
  }, "windows", {
    ZETA_PROFILE_ROOT: "C:\\state",
  });

  assert.equal(environment.PATH, "C:\\Windows\\System32");
  assert.equal(environment.SYSTEMROOT, "C:\\Windows");
  assert.equal(environment.USERPROFILE, "C:\\Users\\zeta");
  assert.equal(environment.AWS_SECRET_ACCESS_KEY, undefined);
});

test("App Server product environment accepts only owned non-NUL variables", () => {
  assert.throws(() => buildAppServerEnvironment({}, "posix", { OPENAI_API_KEY: "secret" }), /Invalid App Server product environment variable/);
  assert.throws(() => buildAppServerEnvironment({}, "posix", { ZETA_PROFILE_ROOT: "bad\0path" }), /Invalid App Server product environment variable/);
});
