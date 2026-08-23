import assert from "node:assert/strict";
import test from "node:test";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { createAsterVisualRangeRectangles } from "../../common/viewModel/visualRangeGeometry.js";
import { createAsterVisualSelectionGeometry } from "../../common/viewModel/visualSelectionGeometry.js";
import { type TextMeasurer } from "../../browser/measurement/fontMetrics.js";

test("visual selection geometry splits one logical range across wrapped fragments", () => {
	using model = new TextModel("abcdef\ngh");
	const projection = EditorVisualLineProjection.fromBreakColumns(model, [[2, 4, 6], [2]]);
	const selections = TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 1), TextPosition.at(1, 1)));
	const geometry = createAsterVisualSelectionGeometry(model, selections, projection, {
		startLineIndex: 0,
		endLineIndexExclusive: 4,
	}, 10, new FixedTextMeasurer());

	assert.deepEqual(geometry.selections, [
		{ selectionIndex: 0, visualLineIndex: 0, left: 20, width: 10 },
		{ selectionIndex: 0, visualLineIndex: 1, left: 10, width: 20 },
		{ selectionIndex: 0, visualLineIndex: 2, left: 10, width: 30 },
		{ selectionIndex: 0, visualLineIndex: 3, left: 10, width: 10 },
	]);
	assert.deepEqual(geometry.carets, [{
		selectionIndex: 0,
		visualLineIndex: 3,
		left: 20,
		primary: true,
	}]);
});

test("visual range geometry rejects a projection from another model version", () => {
	using model = new TextModel("abc");
	const projection = EditorVisualLineProjection.fromBreakColumns(model, [[3]]);
	model.applyEdits([{ range: TextRange.emptyAt(TextPosition.at(0, 3)), text: "d" }]);
	assert.throws(() => createAsterVisualRangeRectangles(model, [{
		range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 1)),
		value: undefined,
	}], projection, { startLineIndex: 0, endLineIndexExclusive: 1 }, 0, new FixedTextMeasurer()), /current text model projection/);
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
