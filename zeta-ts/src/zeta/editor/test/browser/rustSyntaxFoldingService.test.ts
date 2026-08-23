import { strict as assert } from "node:assert";
import test from "node:test";
import { syntaxLanguageForStanzaLanguage } from "../../browser/services/rustSyntaxFactsService.js";
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

test("Rust syntax folding rejects stale revisions and maps every parser-backed Stanza language", () => {
	assert.deepEqual(projectRustSyntaxFoldingRanges({ revision: 2, foldingRanges: [] }, 3), []);
	assert.equal(syntaxLanguageForStanzaLanguage("javascript"), "javascript");
	assert.equal(syntaxLanguageForStanzaLanguage("javascriptreact"), "javascriptreact");
	assert.equal(syntaxLanguageForStanzaLanguage("rust"), "rust");
	assert.equal(syntaxLanguageForStanzaLanguage("typescript"), "typescript");
	assert.equal(syntaxLanguageForStanzaLanguage("typescriptreact"), "typescriptreact");
	assert.equal(syntaxLanguageForStanzaLanguage("markdown"), undefined);
});
