import assert from "node:assert/strict";
import test from "node:test";
import { createAlphaIndentationGuides } from "../../browser/indentation.js";

test("Indentation guides follow complete visual units in mixed leading whitespace", () => {
  assert.deepEqual(createAlphaIndentationGuides("        value", 4), [
    { columnIndex: 4, level: 1 },
    { columnIndex: 8, level: 2 },
  ]);
  assert.deepEqual(createAlphaIndentationGuides("  \t  value", 4), [
    { columnIndex: 3, level: 1 },
  ]);
});

test("Indentation guides stop at source text and validate tab sizing", () => {
  assert.deepEqual(createAlphaIndentationGuides("  value  ", 4), []);
  assert.throws(() => createAlphaIndentationGuides("  ", 0), /positive safe integer/);
});
