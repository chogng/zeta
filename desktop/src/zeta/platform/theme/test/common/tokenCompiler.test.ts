import { strict as assert } from "node:assert";
import test from "node:test";
import { compileDesignTokenArtifacts } from "../../common/tokenCompiler.js";
import { parseUserColorTheme } from "../../common/userColorTheme.js";

test("design token compiler emits deterministic validated artifacts", () => {
  const first = compileDesignTokenArtifacts();
  const second = compileDesignTokenArtifacts();
  assert.deepEqual(first, second);
  const manifest = JSON.parse(first.manifest) as { colors: unknown[]; sizes: unknown[] };
  assert.equal(manifest.colors.length, 61);
  assert.equal(manifest.sizes.length, 6);
  assert.match(first.catalog, /Generated design token catalog/);
  assert.equal(parseUserColorTheme(first.userThemeTemplate).id, "my-custom-theme");
});
