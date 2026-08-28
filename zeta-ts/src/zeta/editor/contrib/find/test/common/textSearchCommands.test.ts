import assert from "node:assert/strict";
import test from "node:test";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { findTextMatches, TextSearchPatternKind } from "../../../../common/model/textModelSearch.js";
import { createReplaceAllTextMatchesCommand, createReplaceTextMatchCommand, resolveTextSearchReplacement } from "../../common/textSearchCommands.js";

test("regular-expression replacement expands captures and named captures", () => {
	using model = new TextModel("name: zeta");
	const match = findTextMatches(model, {
		pattern: "(?<key>name): (zeta)",
		patternKind: TextSearchPatternKind.RegularExpression,
	})[0]!;

	assert.equal(resolveTextSearchReplacement(match, "$<key>=$2 $$ $&"), "name=zeta $ name: zeta");
});

test("replace match is isolated, positions the caret, and rejects stale results", () => {
	using model = new TextModel("one two");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	const match = findTextMatches(model, { pattern: "one" })[0]!;

	selections.execute(createReplaceTextMatchCommand(model, match, "first"));
	assert.equal(model.getText(), "first two");
	assert.deepEqual(selections.selections.primary.active, TextPosition.at(0, 5));
	assert.throws(() => createReplaceTextMatchCommand(model, match, "stale"), /stale model version/);

	selections.undo();
	assert.equal(model.getText(), "one two");
});

test("replace all maps the result caret and undoes as one transaction", () => {
	using model = new TextModel("a a a");
	using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
	const matches = findTextMatches(model, { pattern: "a", matchCase: true });

	selections.execute(createReplaceAllTextMatchesCommand(model, matches, ["long", "", "x"]));
	assert.equal(model.getText(), "long  x");
	assert.deepEqual(selections.selections.primary.active, TextPosition.at(0, 7));

	selections.undo();
	assert.equal(model.getText(), "a a a");
});
