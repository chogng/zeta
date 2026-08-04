import { strict as assert } from "node:assert";
import test from "node:test";
import { normalizeExtensionCatalog } from "../../common/extensionApi.js";

test("normalizes an extension catalog and preserves explicit diagnostics", () => {
  const catalog = normalizeExtensionCatalog({
    generation: 3,
    extensions: [{
      id: "zeta.demo",
      name: "demo",
      publisher: "zeta",
      version: "1.0.0",
      displayName: "Demo",
      sourceKind: "builtIn",
      manifestJson: "{}",
      manifestSha256: "sha256:test",
    }],
    diagnostics: [{
      source: "user",
      subject: null,
      code: "invalidManifest",
      message: "manifest is invalid",
    }],
  });

  assert.equal(catalog.generation, 3);
  assert.equal(catalog.extensions[0]?.id, "zeta.demo");
  assert.equal(catalog.diagnostics[0]?.subject, undefined);
  assert(Object.isFrozen(catalog));
});

test("rejects unknown extension diagnostics", () => {
  assert.throws(() => normalizeExtensionCatalog({
    generation: 1,
    extensions: [],
    diagnostics: [{ source: "user", subject: null, code: "unknown", message: "bad" }],
  }));
});
