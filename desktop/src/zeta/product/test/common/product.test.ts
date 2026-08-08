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
} from "../../../product/common/product.js";
import {
  resolvePackagedProductId,
  resolveProductDataPaths,
} from "../../../product/node/product.js";

test("product selection defaults to the Zeta Desktop build and accepts every edition", () => {
  assert.equal(resolveProductId(undefined), "code");
  assert.equal(resolveProductId(""), "code");
  for (const productId of productIds) {
    const product = getProductConfiguration(productId);
    assert.equal(resolveProductId(productId), productId);
    assert.equal(product.id, productId);
    assert.ok(rendererEntryNames.includes(product.rendererEntry));
    assert.match(product.applicationId, /^com\.zeta\.desktop\./);
    assert.ok(product.userDataFolderName.length > 0);
    assert.equal(product.storageNamespace, product.id);
  }
  assert.equal(new Set(productIds.map((id) => getProductConfiguration(id).applicationId)).size, productIds.length);
  assert.equal(new Set(productIds.map((id) => getProductConfiguration(id).userDataFolderName)).size, productIds.length);
  assert.equal(new Set(productIds.map((id) => getProductConfiguration(id).storageNamespace)).size, productIds.length);
  assert.equal(
    getProductConfiguration("code").rendererEntry,
    "workbench-code",
  );
  assert.equal(getProductConfiguration("code").name, "Zeta");
  assert.equal(
    getProductConfiguration("academic").rendererEntry,
    "workbench-academic",
  );
  assert.equal(
    getProductConfiguration("complete").rendererEntry,
    "workbench-complete",
  );
});

test("product data paths keep installed editions and Chromium session data separate", () => {
  const paths = productIds.map((productId) => resolveProductDataPaths(
    "/application-data",
    getProductConfiguration(productId),
  ));

  assert.equal(paths[0]?.userDataPath, "/application-data/Zeta");
  assert.equal(paths[1]?.userDataPath, "/application-data/Zeta Academic");
  assert.equal(paths[2]?.userDataPath, "/application-data/Zeta Complete");
  assert.equal(new Set(paths.map((value) => value.userDataPath)).size, paths.length);
  assert.equal(new Set(paths.map((value) => value.sessionDataPath)).size, paths.length);
  assert.ok(paths.every((value) => value.sessionDataPath.endsWith("/session-data")));
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
