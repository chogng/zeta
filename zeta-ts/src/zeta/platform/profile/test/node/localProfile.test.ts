import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { migrateLegacyLocalProfile, resolveLocalProfileRoot } from "../../node/localProfile.js";

test("local profile root uses one home-relative convention on every platform", () => {
  assert.equal(resolveLocalProfileRoot({ environment: {}, homeDirectory: "/Users/ada", platform: "darwin" }), "/Users/ada/.zeta");
  assert.equal(resolveLocalProfileRoot({ environment: {}, homeDirectory: "/home/ada", platform: "linux" }), "/home/ada/.zeta");
  assert.equal(resolveLocalProfileRoot({ environment: {}, homeDirectory: "C:\\Users\\ada", platform: "win32" }), "C:\\Users\\ada\\.zeta");
});

test("explicit profile root is authoritative and must be absolute", () => {
  assert.equal(resolveLocalProfileRoot({ environment: { ZETA_PROFILE_ROOT: "/profiles/ada" }, homeDirectory: "/Users/ada", platform: "darwin" }), "/profiles/ada");
  assert.throws(() => resolveLocalProfileRoot({ environment: { ZETA_PROFILE_ROOT: "relative" }, homeDirectory: "/Users/ada", platform: "darwin" }), /absolute path/);
});

test("legacy Desktop resources migrate without overwriting canonical files", async (context) => {
  const root = await mkdtemp(join(tmpdir(), "zeta-local-profile-"));
  context.after(() => rm(root, { recursive: true, force: true }));
  const legacy = join(root, "legacy");
  const profile = join(root, "profile");
  await mkdir(join(legacy, "themes"), { recursive: true });
  await writeFile(join(legacy, "configuration.json"), "legacy", "utf8");
  await writeFile(join(legacy, "keybindings.json"), "bindings", "utf8");
  await writeFile(join(legacy, "themes", "custom.json"), "theme", "utf8");

  await migrateLegacyLocalProfile({ legacyUserDataRoot: legacy, profileRoot: profile });
  assert.equal(await readFile(join(profile, "configuration.json"), "utf8"), "legacy");
  assert.equal(await readFile(join(profile, "themes", "custom.json"), "utf8"), "theme");

  await writeFile(join(profile, "configuration.json"), "canonical", "utf8");
  await migrateLegacyLocalProfile({ legacyUserDataRoot: legacy, profileRoot: profile });
  assert.equal(await readFile(join(profile, "configuration.json"), "utf8"), "canonical");
});
