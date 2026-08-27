import assert from "node:assert/strict";
import { existsSync, readdirSync } from "node:fs";
import { extname, join, resolve } from "node:path";
import test from "node:test";

const repositoryRoot = resolve(import.meta.dirname, "..");

test("repository build orchestration and developer scripts have separate root owners", () => {
  for (const directory of ["zeta-ts/scripts", "docs-site/build", "docs-site/scripts"]) {
    const path = join(repositoryRoot, directory);
    assert.equal(existsSync(path), false, `${directory} must not own repository tooling`);
  }
  for (const category of ["desktop", "docs", "download", "lib", "pnpm", "release", "vite", "app"]) {
    assert.equal(existsSync(join(import.meta.dirname, category)), true, category);
  }
  for (const entry of ["test.ts", "test-documentation.ts", "test-editor.ts", "test-extensions.ts", "test-integration.ts", "test-smoke.ts", "test-web-integration.ts", "test"]) {
    assert.equal(existsSync(join(repositoryRoot, "scripts", entry)), true, entry);
  }
  assert.equal(existsSync(join(repositoryRoot, "docs-site", "app", "generated-docs.ts")), false);
});

test("Node build and developer script sources use TypeScript", () => {
  const javaScriptExtensions = new Set([".cjs", ".js", ".mjs", ".mts"]);
  const files = [...walk(import.meta.dirname), ...walk(join(repositoryRoot, "scripts"))];
  assert.deepEqual(files.filter((path) => javaScriptExtensions.has(extname(path))), []);
});

function walk(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    if (entry.name === "node_modules") return [];
    const path = join(directory, entry.name);
    return entry.isDirectory() ? walk(path) : [path];
  });
}
