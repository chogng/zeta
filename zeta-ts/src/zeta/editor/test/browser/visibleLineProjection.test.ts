import assert from "node:assert/strict";
import test from "node:test";
import { EditorLineWrapping } from "../../common/config/editorOptions.js";
import { ViewModelLines } from "../../common/viewModel/viewModelLines.js";
import { ZetaDOMLineBreaksComputer } from "../../browser/view/zetaDomLineBreaksComputer.js";
import { type TextMeasurer } from "../../browser/config/fontMeasurements.js";
import { EditorFoldingModel } from "../../contrib/folding/browser/foldingModel.js";
import { EditorHiddenRangeModel } from "../../contrib/folding/browser/hiddenRangeModel.js";
import { TextModel } from "../../common/model/textModel.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";

test("Visible visual-line projection removes hidden bodies while preserving wrapped header rows", () => {
	using model = new TextModel("header\ninside\nend\nlast");
	using folding = new EditorFoldingModel(model);
	using hiddenRanges = new EditorHiddenRangeModel(model, folding);
	using projection = new ViewModelLines(model, new ZetaDOMLineBreaksComputer(new FixedTextMeasurer()), {
		wrapping: EditorLineWrapping.On,
		wrapWidth: 20,
		visibilitySource: hiddenRanges,
	});

	assert.deepEqual(projection.projection.lines.map(line => ({ logical: line.logicalLineIndex, start: line.startColumn, end: line.endColumn })), [
		{ logical: 0, start: 0, end: 2 },
		{ logical: 0, start: 2, end: 4 },
		{ logical: 0, start: 4, end: 6 },
		{ logical: 1, start: 0, end: 2 },
		{ logical: 1, start: 2, end: 4 },
		{ logical: 1, start: 4, end: 6 },
		{ logical: 2, start: 0, end: 2 },
		{ logical: 2, start: 2, end: 3 },
		{ logical: 3, start: 0, end: 2 },
		{ logical: 3, start: 2, end: 4 },
	]);

	folding.setRanges([{ startLineIndex: 0, endLineIndex: 2, collapsed: true }]);
	assert.deepEqual(projection.projection.lines.map(line => line.logicalLineIndex), [0, 0, 0, 3, 3]);
	assert.equal(projection.lineSource.lineCount, 5);
	assert.equal(projection.projection.visualLineIndexAt(new Position((1) + 1, (3) + 1)), 2);
	assert.equal(projection.projection.lineAt(2)?.logicalLineIndex, 0);
});

test("Visible visual-line projection refreshes the source before collapsed ranges observe a shrinking model", () => {
	using model = new TextModel("header\ninside\nend");
	using folding = new EditorFoldingModel(model);
	using hiddenRanges = new EditorHiddenRangeModel(model, folding);
	using projection = new ViewModelLines(model, new ZetaDOMLineBreaksComputer(new FixedTextMeasurer()), {
		visibilitySource: hiddenRanges,
	});
	folding.setRanges([{ startLineIndex: 0, endLineIndex: 2, collapsed: true }]);

	assert.doesNotThrow(() => model.applyEdits([{
		range: Range.fromPositions(new Position((0) + 1, (0) + 1), model.positionAt(model.length)),
		text: "x",
	}]));
	assert.equal(projection.projection.logicalLineCount, 1);
	assert.equal(projection.projection.lines.length, 1);
	assert.equal(projection.projection.lineAt(0)?.logicalLineIndex, 0);
});

test("View-model lines keep the wrapping projection path when no visibility filter is installed", () => {
	using model = new TextModel("first\nsecond");
	using projection = new ViewModelLines(model, new ZetaDOMLineBreaksComputer(new FixedTextMeasurer()));

	const initialProjection = projection.projection;
	model.applyEdits([{ range: Range.fromPositions(new Position((0) + 1, (0) + 1), new Position((0) + 1, (0) + 1)), text: "x" }]);
	assert.notEqual(projection.projection, initialProjection);
	assert.equal(projection.projection.visualLineCount, 2);
});

class FixedTextMeasurer implements TextMeasurer {
	readonly horizontalPadding = 0;
	readonly contentLeftPadding = 0;

	refresh(): boolean {
		return false;
	}

	measureLineWidth(text: string): number {
		return text.length * 10;
	}
}
