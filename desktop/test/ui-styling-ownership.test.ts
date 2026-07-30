import { strict as assert } from "node:assert";
import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";
import test from "node:test";

const sharedInteractionSelector = /\.zeta-(?:action-bar|button|tab(?:\b|-)|view-pane(?:\b|-))/;
const ariaStateSelector = /\[aria-(?:checked|pressed|selected)\b/;
const negatedProjectedStateSelector = /:not\(\.(?:active|checked|selected)\)/;

test("Workbench Part CSS does not reach into shared interaction controls", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const violations: string[] = [];
  for (const file of await partCssFiles(sourceRoot)) {
    const source = await readFile(file, "utf8");
    const name = relative(sourceRoot, file).replaceAll("\\", "/");
    for (const [index, line] of source.split(/\r?\n/).entries()) {
      if (sharedInteractionSelector.test(line)) violations.push(`${name}:${index + 1}: ${line.trim()}`);
    }
  }
  assert.deepEqual(violations, []);
});

test("CSS uses state classes instead of ARIA attributes as visual selectors", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const violations: string[] = [];
  for (const file of await cssFiles(sourceRoot)) {
    const source = await readFile(file, "utf8");
    const name = relative(sourceRoot, file).replaceAll("\\", "/");
    for (const [index, line] of source.split(/\r?\n/).entries()) {
      if (ariaStateSelector.test(line)) violations.push(`${name}:${index + 1}: ${line.trim()}`);
    }
  }
  assert.deepEqual(violations, []);
});

test("CSS state precedence does not negate projected state classes", async () => {
  const sourceRoot = join(process.cwd(), "src", "zeta");
  const violations: string[] = [];
  for (const file of await cssFiles(sourceRoot)) {
    const source = await readFile(file, "utf8");
    const name = relative(sourceRoot, file).replaceAll("\\", "/");
    for (const [index, line] of source.split(/\r?\n/).entries()) {
      if (negatedProjectedStateSelector.test(line)) violations.push(`${name}:${index + 1}: ${line.trim()}`);
    }
  }
  assert.deepEqual(violations, []);
});

async function partCssFiles(directory: string): Promise<string[]> {
  return (await cssFiles(directory)).filter((file) => /part\.css$/i.test(file));
}

async function cssFiles(directory: string): Promise<string[]> {
  const result: string[] = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) result.push(...await cssFiles(path));
    else if (entry.name.endsWith(".css")) result.push(path);
  }
  return result;
}
