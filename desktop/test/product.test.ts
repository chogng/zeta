import assert from "node:assert/strict";
import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  getProductConfiguration,
  productIds,
  rendererEntryNames,
  resolveProductId,
} from "../src/zeta/product/common/product.js";
import {
  resolvePackagedProductId,
} from "../src/zeta/product/node/product.js";

test("product selection defaults to Code and accepts every build edition", () => {
  assert.equal(resolveProductId(undefined), "code");
  assert.equal(resolveProductId(""), "code");
  for (const productId of productIds) {
    const product = getProductConfiguration(productId);
    assert.equal(resolveProductId(productId), productId);
    assert.equal(product.id, productId);
    assert.ok(rendererEntryNames.includes(product.rendererEntry));
  }
  assert.equal(
    getProductConfiguration("code").rendererEntry,
    "workbench-code",
  );
  assert.equal(
    getProductConfiguration("academic").rendererEntry,
    "workbench-academic",
  );
  assert.equal(
    getProductConfiguration("complete").rendererEntry,
    "workbench-complete",
  );
});

test("product selection rejects unknown build editions", () => {
  assert.throws(
    () => resolveProductId("enterprise"),
    /Unknown Zeta product 'enterprise'/,
  );
});

test("packaged product selection requires exactly one renderer edition", () => {
  const rendererRoot = mkdtempSync(join(tmpdir(), "zeta-products-"));
  try {
    assert.throws(
      () => resolvePackagedProductId(rendererRoot),
      /found none/,
    );

    createPackagedRenderer(rendererRoot, "academic");
    assert.equal(
      resolvePackagedProductId(rendererRoot),
      "academic",
    );

    createPackagedRenderer(rendererRoot, "code");
    assert.throws(
      () => resolvePackagedProductId(rendererRoot),
      /found code, academic/,
    );
  } finally {
    rmSync(rendererRoot, { force: true, recursive: true });
  }
});

function createPackagedRenderer(
  rendererRoot: string,
  productId: "code" | "academic",
): void {
  const product = getProductConfiguration(productId);
  const workbenchRoot = join(
    rendererRoot,
    productId,
    "electron-browser",
    "workbench",
  );
  mkdirSync(workbenchRoot, { recursive: true });
  writeFileSync(
    join(workbenchRoot, `${product.rendererEntry}.html`),
    "<!doctype html>",
  );
}
