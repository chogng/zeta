import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, readdir, rm, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { cargoArtifactExecutable, cargoRenderedDiagnostic, parseCargoMessage } from "../cargo.ts";
import { publishAppServerGeneration, relativeWatchedDirectory, shouldRebuildAppServer, shouldRebuildWorkspaceManifest } from "./watchAppServer.ts";

test("reads executable paths and diagnostics from Cargo JSON messages", () => {
  const artifact = parseCargoMessage(JSON.stringify({
    executable: "/custom/target/aarch64-apple-darwin/dev-small/zeta-app-server-daemon",
    reason: "compiler-artifact",
    target: { kind: ["bin"], name: "zeta-app-server-daemon" },
  }));
  assert.ok(artifact && typeof artifact === "object");
  assert.equal(cargoArtifactExecutable(artifact, "zeta-app-server-daemon"), "/custom/target/aarch64-apple-darwin/dev-small/zeta-app-server-daemon");
  assert.equal(cargoArtifactExecutable(artifact, "other"), undefined);
  assert.equal(cargoArtifactExecutable({ ...artifact, executable: null }, "zeta-app-server-daemon"), undefined);
  assert.equal(cargoRenderedDiagnostic({ reason: "compiler-message", message: { rendered: "warning\n" } }), "warning\n");
  assert.equal(parseCargoMessage("not JSON"), undefined);
});

test("app-server watcher selects Rust sources and Cargo manifests", () => {
  assert.equal(shouldRebuildAppServer("zeta-rs/app-server/src/main.rs"), true);
  assert.equal(shouldRebuildAppServer("zeta-rs/app-server/build.rs"), true);
  assert.equal(shouldRebuildAppServer("zeta-rs/app-server/Cargo.toml"), true);
  assert.equal(shouldRebuildAppServer("Cargo.lock"), true);
  assert.equal(shouldRebuildAppServer("target/debug/zeta-app-server-daemon"), false);
  assert.equal(shouldRebuildAppServer("target/debug/build/generated/out/schema.rs"), false);
  assert.equal(shouldRebuildAppServer("crate/target/debug/build/generated/out/schema.rs"), false);
  assert.equal(shouldRebuildAppServer("zeta-ts/src/main.ts"), false);
});

test("app-server watcher excludes a custom Cargo target directory inside Rust sources", () => {
  const sourceRoot = join("/workspace", "zeta", "zeta-rs");
  const customTarget = join(sourceRoot, ".cargo-cache");
  const ignored = relativeWatchedDirectory(sourceRoot, customTarget);
  assert.equal(ignored, ".cargo-cache");
  assert.equal(shouldRebuildAppServer(".cargo-cache/debug/build/codegen/out/generated.rs", ignored), false);
  assert.equal(shouldRebuildAppServer("app-server/src/main.rs", ignored), true);
  assert.equal(relativeWatchedDirectory(sourceRoot, join("/workspace", "zeta", "target")), undefined);
});

test("workspace-root watcher accepts only canonical root manifests", () => {
  assert.equal(shouldRebuildWorkspaceManifest("Cargo.toml"), true);
  assert.equal(shouldRebuildWorkspaceManifest("Cargo.lock"), true);
  assert.equal(shouldRebuildWorkspaceManifest("app/main.rs"), false);
  assert.equal(shouldRebuildWorkspaceManifest("target/debug/build/generated/out/schema.rs"), false);
  assert.equal(shouldRebuildWorkspaceManifest("zeta-rs/app-server/Cargo.toml"), false);
});

test("app-server publisher reuses identical content and retains one rollback generation", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-app-server-daemon-publisher-"));
  const source = join(root, "target", "debug", "zeta-app-server-daemon");
  const generations = join(root, "generations");
  const pointer = join(generations, "current.json");
  try {
    await mkdir(join(root, "target", "debug"), { recursive: true });
    await writeFile(source, "one");
    const first = await publishAppServerGeneration(source, generations, pointer, "darwin");
    assert.equal(first.changed, true);
    assert.match(first.generation, /^zeta-app-server-daemon\.[a-f0-9]{64}$/u);
    assert.equal((await readdir(generations)).filter(name => name.startsWith("zeta-app-server-daemon.")).length, 1);

    const unchanged = await publishAppServerGeneration(source, generations, pointer, "darwin");
    assert.deepEqual(unchanged, { changed: false, generation: first.generation });
    assert.equal((await readdir(generations)).filter(name => name.startsWith("zeta-app-server-daemon.")).length, 1);

    await writeFile(source, "two");
    const second = await publishAppServerGeneration(source, generations, pointer, "darwin");
    await utimes(join(generations, first.generation), new Date(1_000), new Date(1_000));
    await utimes(join(generations, second.generation), new Date(2_000), new Date(2_000));
    await writeFile(source, "three");
    const third = await publishAppServerGeneration(source, generations, pointer, "darwin");
    const published = (await readdir(generations)).filter(name => name.startsWith("zeta-app-server-daemon.")).sort();
    assert.deepEqual(published, [second.generation, third.generation].sort());
    assert.deepEqual(JSON.parse(await readFile(pointer, "utf8")), { version: 1, executable: third.generation });
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("app-server publisher removes duplicate legacy generations while preserving distinct rollback content", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-app-server-daemon-publisher-"));
  const source = join(root, "target", "debug", "zeta-app-server-daemon");
  const generations = join(root, "generations");
  const pointer = join(generations, "current.json");
  try {
    await mkdir(join(root, "target", "debug"), { recursive: true });
    await mkdir(generations);
    await writeFile(source, "current");
    await writeFile(join(generations, "zeta-app-server-daemon.100.0"), "rollback");
    await writeFile(join(generations, "zeta-app-server-daemon.200.0"), "current");
    await writeFile(join(generations, "zeta-app-server-daemon.300.0"), "current");
    await writeFile(pointer, `${JSON.stringify({ version: 1, executable: "zeta-app-server-daemon.300.0" })}\n`);

    const published = await publishAppServerGeneration(source, generations, pointer, "darwin");
    const files = (await readdir(generations)).filter(name => name.startsWith("zeta-app-server-daemon.")).sort();
    assert.deepEqual(files, ["zeta-app-server-daemon.100.0", published.generation].sort());
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});
