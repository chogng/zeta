import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { lineEndPosition, lineStartPosition, nextCaretPosition, previousCaretPosition } from "../../common/cursor/caretOperations.js";
import { getWordPartRangeAtPosition, getWordPartRanges } from "../../common/cursor/wordPartOperations.js";
import { getWordRangeAtPosition, nextWordPosition, previousWordPosition } from "../../common/cursor/wordOperations.js";

test("caret operations preserve UTF-16 boundaries and cross logical lines", () => {
  using model = new TextModel("A😀\nnext");
  assert.deepEqual(previousCaretPosition(model, TextPosition.at(0, 3)), TextPosition.at(0, 1));
  assert.deepEqual(nextCaretPosition(model, TextPosition.at(0, 1)), TextPosition.at(0, 3));
  assert.deepEqual(previousCaretPosition(model, TextPosition.at(1, 0)), TextPosition.at(0, 3));
  assert.deepEqual(nextCaretPosition(model, TextPosition.at(0, 3)), TextPosition.at(1, 0));
  assert.deepEqual(lineStartPosition(model, TextPosition.at(1, 2)), TextPosition.at(1, 0));
  assert.deepEqual(lineEndPosition(model, TextPosition.at(0, 0)), TextPosition.at(0, 3));
});

test("word operations use model ranges and remain document bounded", () => {
  using model = new TextModel("one two\nthree");
  assert.deepEqual(getWordRangeAtPosition(model, TextPosition.at(0, 1)), TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 3)));
  assert.deepEqual(previousWordPosition(model, TextPosition.at(0, 7)), TextPosition.at(0, 4));
  assert.deepEqual(nextWordPosition(model, TextPosition.at(0, 0)), TextPosition.at(0, 4));
  assert.deepEqual(previousWordPosition(model, TextPosition.at(0, 0)), TextPosition.at(0, 0));
  assert.deepEqual(nextWordPosition(model, TextPosition.at(1, 5)), TextPosition.at(1, 5));
});

test("word part operations split identifiers, digits, and separators", () => {
  assert.deepEqual(getWordPartRanges("HTTPServer42_value").map(range => "HTTPServer42_value".slice(range.start, range.end)), ["HTTP", "Server", "42", "_", "value"]);
  using model = new TextModel("HTTPServer42_value");
  assert.deepEqual(getWordPartRangeAtPosition(model, TextPosition.at(0, 6)), TextRange.from(TextPosition.at(0, 4), TextPosition.at(0, 10)));
});
