import { strict as assert } from "node:assert";
import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";
import test from "node:test";
import { compileDesignTokenArtifacts } from "../src/zeta/platform/theme/common/tokenCompiler.js";

test("CSS consumes registered design tokens and isolates intentional color samples", async () => {
  const manifest = JSON.parse(compileDesignTokenArtifacts().manifest) as {
    colors: Array<{ cssVariable: string }>;
    sizes: Array<{ cssVariable: string }>;
  };
  const registered = new Set([...manifest.colors, ...manifest.sizes].map(({ cssVariable }) => cssVariable));
  const platformVariables = new Set(["--zeta-font-family", "--zeta-font-family-monospace"]);
  const intentionalColorFiles = new Set([
    "base/browser/ui/icon/icon.css",
  ]);
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const unknownVariables: string[] = [];
  const rawColors: string[] = [];
  for (const file of await cssFiles(sourceRoot)) {
    const source = await readFile(file, "utf8");
    const name = relative(sourceRoot, file).replaceAll("\\", "/");
    for (const match of source.matchAll(/var\((--zeta-[a-zA-Z0-9-]+)/g)) {
      if (!registered.has(match[1]!) && !platformVariables.has(match[1]!)) unknownVariables.push(`${name}: ${match[1]}`);
    }
    if (!intentionalColorFiles.has(name)) {
      for (const [index, line] of source.split(/\r?\n/).entries()) {
        if (/#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/.test(line)) rawColors.push(`${name}:${index + 1}`);
      }
    }
  }
  assert.deepEqual(unknownVariables, []);
  assert.deepEqual(rawColors, []);
});

async function cssFiles(directory: string): Promise<string[]> {
  const result: string[] = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) result.push(...await cssFiles(path));
    else if (entry.name.endsWith(".css")) result.push(path);
  }
  return result;
}
