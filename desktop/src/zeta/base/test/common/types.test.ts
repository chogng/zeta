import assert from "node:assert/strict";
import test from "node:test";
import { assert as assertCondition, assertDefined } from "../../common/types.js";

test("assert narrows caller-defined conditions", () => {
  assert.equal(requireStringFromUnknown("value"), "value");
  assert.throws(
    () => assertCondition(false, "condition was not satisfied"),
    new Error("condition was not satisfied"),
  );
});

test("assertDefined narrows non-nullable values", () => {
  assert.equal(requireString("value"), "value");
  assert.doesNotThrow(() => assertDefined(false, "missing boolean"));
  assert.doesNotThrow(() => assertDefined(0, "missing number"));
  assert.doesNotThrow(() => assertDefined("", "missing string"));
});

test("assertDefined rejects nullish values with caller context", () => {
  assert.throws(
    () => assertDefined(undefined, "value was not initialized"),
    new Error("value was not initialized"),
  );
  assert.throws(
    () => assertDefined(null, "value was cleared"),
    new Error("value was cleared"),
  );
});

test("assertDefined preserves caller-owned errors", () => {
  const error = new ReferenceError("value was not initialized");
  assert.throws(() => assertDefined(undefined, error), (thrown) => thrown === error);
});

function requireString(value: string | undefined): string {
  assertDefined(value, "string was not initialized");
  return value;
}

function requireStringFromUnknown(value: unknown): string {
  assertCondition(typeof value === "string", "value is not a string");
  return value;
}
