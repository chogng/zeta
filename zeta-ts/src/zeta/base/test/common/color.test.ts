import { strict as assert } from "node:assert";
import test from "node:test";
import { Color } from "../../common/color.js";

test("Color parses, normalizes, and composes immutable RGBA values", () => {
  assert.equal(Color.fromHex("#fff").toString(), "#ffffff");
  assert.equal(Color.fromHex("#33669980").toString(), "#33669980");
  assert.equal(Color.fromHex("#ff0000").transparent(0.5).toString(), "#ff000080");
  assert.equal(Color.fromHex("#000000").lighten(0.5).toString(), "#808080");
  assert.equal(Color.fromHex("#ffffff").darken(0.5).toString(), "#808080");
  assert.equal(Color.fromHex("#ffffff80").makeOpaque(Color.fromHex("#000000")).toString(), "#808080");
});

test("Color rejects malformed values", () => {
  assert.throws(() => Color.fromHex("red"), /Invalid hexadecimal color/);
  assert.throws(() => Color.fromHex("#12"), /Invalid hexadecimal color/);
});
