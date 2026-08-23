import assert from "node:assert/strict";
import { existsSync, readdirSync } from "node:fs";
import { extname, join, resolve } from "node:path";
import test from "node:test";

const repositoryRoot = resolve(import.meta.dirname, "..");

test("repository build orchestration is owned by the root build directory", () => {
  for (const directory of ["desktop/scripts", "docs-site/build", "docs-site/scripts", "scripts"]) {
    const path = join(repositoryRoot, directory);
    assert.deepEqual(existsSync(path) ? readdirSync(path) : [], [], `${directory} must not regain build scripts`);
  }
  for (const category of ["desktop", "docs", "download", "lib", "pnpm", "release", "test", "vite", "zeterm"]) {
    assert.equal(existsSync(join(import.meta.dirname, category)), true, category);
  }
  assert.equal(existsSync(join(repositoryRoot, "docs-site", "app", "generated-docs.ts")), false);
});

test("Node build sources use TypeScript", () => {
  const javaScriptExtensions = new Set([".cjs", ".js", ".mjs", ".mts"]);
  const files = walk(import.meta.dirname);
  assert.deepEqual(files.filter((path) => javaScriptExtensions.has(extname(path))), []);
});

function walk(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    if (entry.name === "node_modules") return [];
    const path = join(directory, entry.name);
    return entry.isDirectory() ? walk(path) : [path];
  });
}
