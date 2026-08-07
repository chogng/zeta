import { strict as assert } from "node:assert";
import test from "node:test";
import { syntaxLanguageForAlphaLanguage } from "../../browser/services/rustSyntaxFactsService.js";
import { projectRustSyntaxFoldingRanges } from "../../browser/services/rustSyntaxFoldingService.js";

test("Rust syntax folding projects matching parser revisions", () => {
  const ranges = projectRustSyntaxFoldingRanges({
    revision: 7,
    foldingRanges: [
      { range: { start: { lineIndex: 1, columnIndex: 0 }, end: { lineIndex: 4, columnIndex: 1 } } },
      { range: { start: { lineIndex: 5, columnIndex: 0 }, end: { lineIndex: 5, columnIndex: 1 } } },
    ],
  }, 7);

  assert.deepEqual(ranges, [{
    startLineIndex: 1,
    endLineIndex: 4,
    collapsed: false,
    source: "provider",
  }]);
});

test("Rust syntax folding rejects stale revisions and maps only supported Alpha languages", () => {
  assert.deepEqual(projectRustSyntaxFoldingRanges({ revision: 2, foldingRanges: [] }, 3), []);
  assert.equal(syntaxLanguageForAlphaLanguage("rust"), "rust");
  assert.equal(syntaxLanguageForAlphaLanguage("typescript"), undefined);
});
