import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition } from "../../common/core/position.js";
import { TextRange } from "../../common/core/range.js";
import { StringEdit, StringReplacement } from "../../common/core/edits/stringEdit.js";
import { LineEdit, LineReplacement } from "../../common/core/edits/lineEdit.js";
import { TextEdit } from "../../common/core/edits/textEdit.js";
import { LineRange } from "../../common/core/ranges/lineRange.js";
import { OffsetRange } from "../../common/core/ranges/offsetRange.js";
import { PositionOffsetTransformer } from "../../common/core/text/positionToOffsetImpl.js";
import { TextLength } from "../../common/core/text/textLength.js";
import { StringText } from "../../common/core/text/abstractText.js";
import { TextChange } from "../../common/core/textChange.js";

const position = TextPosition.at;

test("core converts UTF-16 positions and text lengths", () => {
  const transformer = new PositionOffsetTransformer("A😀\nbeta");
  assert.equal(transformer.getOffset(position(0, 3)), 3);
  assert.deepEqual(transformer.getPosition(2), position(0, 2));
  assert.deepEqual(transformer.getPosition(transformer.text.length), position(1, 4));
  assert.deepEqual(TextLength.ofText("ab\ncd"), new TextLength(1, 2));
  assert.deepEqual(TextLength.ofText("ab\ncd").addToPosition(position(3, 4)), position(4, 2));
});

test("core edit algebra applies, composes, and inverts string edits", () => {
  const first = StringEdit.replace(new OffsetRange(1, 3), "X");
  const second = StringEdit.replace(new OffsetRange(1, 3), "YZ");
  const composed = first.compose(second);
  assert.equal(first.apply("abcdef"), "aXdef");
  assert.equal(second.apply(first.apply("abcdef")), "aYZef");
  assert.equal(composed.apply("abcdef"), "aYZef");
  assert.equal(composed.inverse("abcdef").apply(composed.apply("abcdef")), "abcdef");
  assert.equal(StringEdit.create([StringReplacement.insert(0, "a"), StringReplacement.insert(1, "b")]).apply(""), "ab");
});

test("text edits map coordinate ranges and preserve multiline text", () => {
  const source = new StringText("alpha\nbeta");
  const edit = TextEdit.replace(TextRange.from(position(0, 1), position(0, 3)), "X\nY");
  assert.equal(edit.apply(source), "aX\nYha\nbeta");
  assert.deepEqual(edit.getNewRanges(), [TextRange.from(position(0, 1), position(1, 1))]);
  assert.deepEqual(edit.mapRange(TextRange.from(position(0, 0), position(0, 5))), TextRange.from(position(0, 0), position(1, 3)));
  const second = TextEdit.replace(TextRange.from(position(1, 0), position(1, 1)), "Z");
  assert.equal(edit.compose(second).apply(source), "aX\nZha\nbeta");
});

test("line edits replace line spans and can invert against original lines", () => {
  const edit = new LineEdit([new LineReplacement(new LineRange(1, 2), ["B", "C"])]);
  const original = ["A", "b", "D"];
  assert.deepEqual(edit.apply(original), ["A", "B", "C", "D"]);
  assert.deepEqual(edit.inverse(original).apply(edit.apply(original)), original);

  const source = new StringText(original.join("\n"));
  assert.equal(new LineReplacement(new LineRange(1, 2), []).toSingleEdit(source).replace(source.value), "A\nD");
  assert.equal(new LineReplacement(new LineRange(1, 3), []).toSingleEdit(source).replace(source.value), "A");
  assert.equal(new LineReplacement(new LineRange(0, 1), []).toSingleEdit(source).replace(source.value), "b\nD");
  assert.equal(new LineReplacement(new LineRange(1, 3), ["B", "C"]).toSingleEdit(source).replace(source.value), "A\nB\nC");
  assert.equal(new LineReplacement(new LineRange(3, 3), ["E"]).toSingleEdit(source).replace(source.value), "A\nb\nD\nE");
});

test("text changes round-trip through the compact binary shape", () => {
  const change = new TextChange(3, "old", 4, "new\n");
  const buffer = new Uint8Array(change.writeSize());
  const end = change.write(buffer, 0);
  const decoded: TextChange[] = [];
  assert.equal(TextChange.read(buffer, 0, decoded), end);
  assert.equal(decoded[0]!.toString(), change.toString());
});
