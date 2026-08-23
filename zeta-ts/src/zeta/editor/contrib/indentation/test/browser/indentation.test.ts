import { strict as assert } from "node:assert";
import test from "node:test";
import { EditorIndentationKind, getLeadingIndentation, normalizeEditorIndentation, normalizeEditorIndentationText, resolveEditorIndentationOptions, shiftEditorIndentation, unshiftEditorIndentation } from "../../../../common/editorIndentation.js";

test("Editor indentation resolves spaces and maps mixed whitespace by visual columns", () => {
	const options = resolveEditorIndentationOptions({
		kind: EditorIndentationKind.Spaces,
		tabSize: 4,
	});

	assert.equal(normalizeEditorIndentation("\t  ", options), "      ");
	assert.equal(normalizeEditorIndentationText("\t * value", options), "     * value");
	assert.equal(shiftEditorIndentation("  ", options), "      ");
	assert.equal(unshiftEditorIndentation("      ", options), "  ");
	assert.equal(getLeadingIndentation("  value", 1), " ");
});

test("Editor indentation emits canonical tabs and validates its caller contract", () => {
	const options = resolveEditorIndentationOptions({
		kind: EditorIndentationKind.Tabs,
		tabSize: 4,
	});

	assert.equal(normalizeEditorIndentation("        ", options), "\t\t");
	assert.equal(normalizeEditorIndentation("      ", options), "\t  ");
	assert.equal(shiftEditorIndentation("\t  ", options), "\t\t  ");
	assert.equal(unshiftEditorIndentation("\t  ", options), "  ");
	assert.throws(() => resolveEditorIndentationOptions({ kind: "mixed" as EditorIndentationKind }), /kind/);
	assert.throws(() => resolveEditorIndentationOptions({ tabSize: 0 }), /between 1 and 32/);
	assert.throws(() => normalizeEditorIndentation(" x", options), /tabs and spaces/);
});
