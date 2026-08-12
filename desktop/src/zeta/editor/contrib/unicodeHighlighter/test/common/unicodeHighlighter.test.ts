import assert from "node:assert/strict";
import test from "node:test";
import { findUnicodeHighlights } from "../../common/unicodeHighlighter.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Unicode highlighter finds invisible, bidi, and mixed-script confusable characters", () => {
  using model = new TextModel("const a = 1;\u200b\nconst а = 2;\u202e");
  assert.deepEqual(findUnicodeHighlights(model).map(highlight => highlight.kind), ["invisible", "confusable", "bidi"]);
});
