import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../../../base/common/uri.js";
import { Range } from "../../common/core/range.js";
import { TextEditHistoryGroup } from "../../common/core/editOperation.js";
import { TextModel } from "../../common/model/textModel.js";
import { RetainedModelUndoRedoHistory } from "../../common/services/retainedModelUndoRedoHistory.js";

test("RetainedModelUndoRedoHistory retains only the configured number of model histories", () => {
	using participant = new RetainedModelUndoRedoHistory({ maxEntries: 1 });
	const firstResource = URI.file("C:\\project\\first.ts");
	const secondResource = URI.file("C:\\project\\second.ts");
	using first = editedModel("first");
	using second = editedModel("second");
	participant.remember(firstResource, first);
	participant.remember(secondResource, second);

	using reopenedFirst = new TextModel(first.getText());
	using reopenedSecond = new TextModel(second.getText());
	assert.equal(participant.restore(firstResource, reopenedFirst), false);
	assert.equal(participant.restore(secondResource, reopenedSecond), true);
	assert.equal(reopenedSecond.canUndo(), true);
});

test("RetainedModelUndoRedoHistory does not capture an unfinished history revision", () => {
	using participant = new RetainedModelUndoRedoHistory();
	const resource = URI.file("C:\\project\\revision.ts");
	using model = new TextModel("revision");
	const group = TextEditHistoryGroup.create();
	model.beginHistoryRevision(group);
	model.applyEdits(
		[{ range: Range.fromPositions(model.positionAt(model.length)), text: "!" }],
		{ historyGroup: group },
	);
	participant.remember(resource, model);
	assert.equal(model.finishHistoryRevision(group), true);

	using reopened = new TextModel(model.getText());
	assert.equal(participant.restore(resource, reopened), false);
	assert.equal(reopened.canUndo(), false);
});

function editedModel(text: string): TextModel {
	const model = new TextModel(text);
	model.applyEdits([{ range: Range.fromPositions(model.positionAt(model.length)), text: "!" }]);
	return model;
}
