import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, readdir, rm, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { cargoArtifactExecutable, cargoRenderedDiagnostic, parseCargoMessage } from "../cargo.ts";
import { publishServerHostGeneration, relativeWatchedDirectory, shouldRebuildServerHost, shouldRebuildWorkspaceManifest } from "./watchServerHost.ts";

test("reads executable paths and diagnostics from Cargo JSON messages", () => {
  const artifact = parseCargoMessage(JSON.stringify({
    executable: "/custom/target/aarch64-apple-darwin/dev-small/zeta-server",
    reason: "compiler-artifact",
    target: { kind: ["bin"], name: "zeta-server" },
  }));
  assert.ok(artifact && typeof artifact === "object");
  assert.equal(cargoArtifactExecutable(artifact, "zeta-server"), "/custom/target/aarch64-apple-darwin/dev-small/zeta-server");
  assert.equal(cargoArtifactExecutable(artifact, "other"), undefined);
  assert.equal(cargoArtifactExecutable({ ...artifact, executable: null }, "zeta-server"), undefined);
  assert.equal(cargoRenderedDiagnostic({ reason: "compiler-message", message: { rendered: "warning\n" } }), "warning\n");
  assert.equal(parseCargoMessage("not JSON"), undefined);
});

test("server-host watcher selects Rust sources and Cargo manifests", () => {
  assert.equal(shouldRebuildServerHost("zeta-rs/server-host/src/main.rs"), true);
  assert.equal(shouldRebuildServerHost("zeta-rs/server-host/build.rs"), true);
  assert.equal(shouldRebuildServerHost("zeta-rs/server-host/Cargo.toml"), true);
  assert.equal(shouldRebuildServerHost("Cargo.lock"), true);
  assert.equal(shouldRebuildServerHost("target/debug/zeta-server"), false);
  assert.equal(shouldRebuildServerHost("target/debug/build/generated/out/schema.rs"), false);
  assert.equal(shouldRebuildServerHost("crate/target/debug/build/generated/out/schema.rs"), false);
  assert.equal(shouldRebuildServerHost("zeta-ts/src/main.ts"), false);
});

test("server-host watcher excludes a custom Cargo target directory inside Rust sources", () => {
  const sourceRoot = join("/workspace", "zeta", "zeta-rs");
  const customTarget = join(sourceRoot, ".cargo-cache");
  const ignored = relativeWatchedDirectory(sourceRoot, customTarget);
  assert.equal(ignored, ".cargo-cache");
  assert.equal(shouldRebuildServerHost(".cargo-cache/debug/build/codegen/out/generated.rs", ignored), false);
  assert.equal(shouldRebuildServerHost("server-host/src/main.rs", ignored), true);
  assert.equal(relativeWatchedDirectory(sourceRoot, join("/workspace", "zeta", "target")), undefined);
});

test("workspace-root watcher accepts only canonical root manifests", () => {
  assert.equal(shouldRebuildWorkspaceManifest("Cargo.toml"), true);
  assert.equal(shouldRebuildWorkspaceManifest("Cargo.lock"), true);
  assert.equal(shouldRebuildWorkspaceManifest("zeterm/src/main.rs"), false);
  assert.equal(shouldRebuildWorkspaceManifest("target/debug/build/generated/out/schema.rs"), false);
  assert.equal(shouldRebuildWorkspaceManifest("zeta-rs/app-server/Cargo.toml"), false);
});

test("server-host publisher reuses identical content and retains one rollback generation", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-server-host-publisher-"));
  const source = join(root, "target", "debug", "zeta-server");
  const generations = join(root, "generations");
  const pointer = join(generations, "current.json");
  try {
    await mkdir(join(root, "target", "debug"), { recursive: true });
    await writeFile(source, "one");
    const first = await publishServerHostGeneration(source, generations, pointer, "darwin");
    assert.equal(first.changed, true);
    assert.match(first.generation, /^zeta-server\.[a-f0-9]{64}$/u);
    assert.equal((await readdir(generations)).filter(name => name.startsWith("zeta-server.")).length, 1);

    const unchanged = await publishServerHostGeneration(source, generations, pointer, "darwin");
    assert.deepEqual(unchanged, { changed: false, generation: first.generation });
    assert.equal((await readdir(generations)).filter(name => name.startsWith("zeta-server.")).length, 1);

    await writeFile(source, "two");
    const second = await publishServerHostGeneration(source, generations, pointer, "darwin");
    await utimes(join(generations, first.generation), new Date(1_000), new Date(1_000));
    await utimes(join(generations, second.generation), new Date(2_000), new Date(2_000));
    await writeFile(source, "three");
    const third = await publishServerHostGeneration(source, generations, pointer, "darwin");
    const published = (await readdir(generations)).filter(name => name.startsWith("zeta-server.")).sort();
    assert.deepEqual(published, [second.generation, third.generation].sort());
    assert.deepEqual(JSON.parse(await readFile(pointer, "utf8")), { version: 1, executable: third.generation });
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});

test("server-host publisher removes duplicate legacy generations while preserving distinct rollback content", async () => {
  const root = await mkdtemp(join(tmpdir(), "zeta-server-host-publisher-"));
  const source = join(root, "target", "debug", "zeta-server");
  const generations = join(root, "generations");
  const pointer = join(generations, "current.json");
  try {
    await mkdir(join(root, "target", "debug"), { recursive: true });
    await mkdir(generations);
    await writeFile(source, "current");
    await writeFile(join(generations, "zeta-server.100.0"), "rollback");
    await writeFile(join(generations, "zeta-server.200.0"), "current");
    await writeFile(join(generations, "zeta-server.300.0"), "current");
    await writeFile(pointer, `${JSON.stringify({ version: 1, executable: "zeta-server.300.0" })}\n`);

    const published = await publishServerHostGeneration(source, generations, pointer, "darwin");
    const files = (await readdir(generations)).filter(name => name.startsWith("zeta-server.")).sort();
    assert.deepEqual(files, ["zeta-server.100.0", published.generation].sort());
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});
