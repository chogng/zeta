import assert from "node:assert/strict";
import test from "node:test";
import {
  isMacintosh,
  isWindows,
} from "../../../base/common/platform.js";
import {
  MONACO_EDITOR_FONT_DEFAULTS,
  MonacoEditorFontConfiguration,
  readMonacoEditorFontSettings,
} from "../common/config/editorConfiguration.js";

test("Monaco font settings use its platform defaults", () => {
  const expectedFamily = isMacintosh
    ? "Menlo, Monaco, 'Courier New', monospace"
    : isWindows
      ? "Consolas, 'Courier New', monospace"
      : "'Droid Sans Mono', monospace";

  assert.deepEqual(readMonacoEditorFontSettings(), {
    ...MONACO_EDITOR_FONT_DEFAULTS,
    fontFamily: expectedFamily,
  });
});

test("Monaco font configuration validates persisted values", () => {
  assert.equal(
    MonacoEditorFontConfiguration.fontFamily.parse("  Cascadia Code  "),
    "Cascadia Code",
  );
  assert.equal(
    MonacoEditorFontConfiguration.fontWeight.parse("1001"),
    "1000",
  );
  assert.equal(
    MonacoEditorFontConfiguration.fontSize.parse(120),
    100,
  );
  assert.equal(
    MonacoEditorFontConfiguration.lineHeight.parse(1.5),
    1.5,
  );
  assert.equal(
    MonacoEditorFontConfiguration.fontLigatures.parse("'calt'"),
    "'calt'",
  );
  assert.throws(
    () => MonacoEditorFontConfiguration.fontWeight.parse("semibold"),
    /editor\.fontWeight/,
  );
  assert.throws(
    () => MonacoEditorFontConfiguration.fontSize.parse("14"),
    /editor\.fontSize/,
  );
});
