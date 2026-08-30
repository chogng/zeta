import { strict as assert } from "node:assert";
import test from "node:test";
import { EditorFoldingRangeSource } from "../../browser/foldingRanges.js";
import { computeEditorIndentFoldingRanges } from "../../browser/indentRangeProvider.js";
import { registerBuiltinLanguageConfigurations } from "../../../../common/languages/languageBuiltinConfigurations.js";
import { TestLanguageConfigurationService } from '../../../../test/common/modes/testLanguageConfigurationService.js';
import { computeEditorLanguageFoldingRanges, mergeEditorFoldingRanges } from "../../browser/syntaxRangeProvider.js";
import { TextModel } from "../../../../common/model/textModel.js";

test("Language folding follows lexical braces, brackets, and block comments", () => {
	using model = new TextModel("const matcher = /[{]/;\nfunction sample() {\nvalues = [\n1,\n];\n}\n/*\ncomment {\n*/");
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);

	assert.deepEqual(computeEditorLanguageFoldingRanges(model, "typescript", configurations), [
		range(1, 5),
		range(2, 4),
		range(6, 8),
	]);
});

test("Language folding recognizes nested configured comment regions without mistaking source text for a marker", () => {
	using model = new TextModel("// #region outer\nconst marker = '#region';\n// region inner\nvalue();\n// endregion inner\n// #endregion outer");
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);

	assert.deepEqual(computeEditorLanguageFoldingRanges(model, "typescript", configurations), [
		range(0, 5),
		range(2, 4),
	]);
});

test("Language folding accepts caller-contributed region marker patterns", () => {
	using model = new TextModel("; region first\nvalue\n; endregion first");
	using configurations = new TestLanguageConfigurationService();
	using registration = configurations.register("demo", {
		folding: { markers: { start: /^\s*;\s*region\b/iu, end: /^\s*;\s*endregion\b/iu } },
	});

	assert.deepEqual(computeEditorLanguageFoldingRanges(model, "demo", configurations), [range(0, 2)]);
});

test("Language and indentation folds merge deterministically without crossing ranges", () => {
	using model = new TextModel("if {\n  child\n}\nplain\n  indented");
	using configurations = new TestLanguageConfigurationService();
	using builtins = registerBuiltinLanguageConfigurations(configurations);

	assert.deepEqual(mergeEditorFoldingRanges(
		computeEditorLanguageFoldingRanges(model, "typescript", configurations),
		computeEditorIndentFoldingRanges(model),
	), [
		range(0, 2),
		range(3, 4),
	]);
});

function range(startLineIndex: number, endLineIndex: number): { readonly startLineIndex: number; readonly endLineIndex: number; readonly collapsed: false; readonly source: EditorFoldingRangeSource } {
	return Object.freeze({ startLineIndex, endLineIndex, collapsed: false, source: EditorFoldingRangeSource.Provider });
}
