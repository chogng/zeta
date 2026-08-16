import assert from "node:assert/strict";
import test from "node:test";

import { parseRustHostTriple, shouldRebuildServerHost } from "./watch.mjs";

test("server-host watcher selects Rust sources and Cargo manifests", () => {
  assert.equal(shouldRebuildServerHost("zeta-rs/server-host/src/main.rs"), true);
  assert.equal(shouldRebuildServerHost("zeta-rs/server-host/build.rs"), true);
  assert.equal(shouldRebuildServerHost("zeta-rs/server-host/Cargo.toml"), true);
  assert.equal(shouldRebuildServerHost("Cargo.lock"), true);
  assert.equal(shouldRebuildServerHost("target/debug/zeta-server"), false);
  assert.equal(shouldRebuildServerHost("desktop/src/main.ts"), false);
});

test("server-host watcher reads rustc's exact host target", () => {
  assert.equal(parseRustHostTriple("rustc 1.92.0\nhost: aarch64-apple-darwin\nLLVM version: 21\n"), "aarch64-apple-darwin");
  assert.throws(() => parseRustHostTriple("rustc 1.92.0\n"), /host target triple/u);
});
