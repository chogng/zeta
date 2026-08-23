import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { hitTestStanzaVisualEditorPoint, EditorHitTargetKind } from "../../common/viewModel/pointerHitTest.js";
import { type TextMeasurer } from "../../browser/measurement/fontMetrics.js";

test("visual hit testing maps wrapped visual coordinates back to logical UTF-16 positions", () => {
	using model = new TextModel("abcdef\ngh");
	const projection = EditorVisualLineProjection.fromBreakColumns(model, [[2, 4, 6], [2]]);
	const layout = {
		lineHeight: 20,
		viewportSize: { width: 200, height: 80 },
		scrollPosition: { left: 0, top: 0 },
	};
	const metrics = { gutterWidth: 30, textLeft: 40 };
	const measurer = new FixedTextMeasurer();

	assert.deepEqual(hitTestStanzaVisualEditorPoint(model, projection, layout, { left: 52, top: 25 }, metrics, measurer), {
		kind: EditorHitTargetKind.Text,
		position: TextPosition.at(0, 3),
	});
	assert.deepEqual(hitTestStanzaVisualEditorPoint(model, projection, layout, { left: 100, top: 45 }, metrics, measurer), {
		kind: EditorHitTargetKind.EmptyContent,
		position: TextPosition.at(0, 6),
	});
	assert.deepEqual(hitTestStanzaVisualEditorPoint(model, projection, layout, { left: 10, top: 65 }, metrics, measurer), {
		kind: EditorHitTargetKind.Gutter,
		position: TextPosition.at(1, 0),
	});
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
