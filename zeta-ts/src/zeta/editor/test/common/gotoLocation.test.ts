import assert from "node:assert/strict";
import test from "node:test";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { parseStanzaGotoLocation } from "../../common/commands/gotoLocation.js";

test("Go to Line parses one-based line and column inputs with clamping", () => {
	using model = new TextModel("zero\none\ntwo");
	assertLocation(parseStanzaGotoLocation(model, "2"), TextPosition.at(1, 0));
	assertLocation(parseStanzaGotoLocation(model, ":2:3"), TextPosition.at(1, 2));
	assertLocation(parseStanzaGotoLocation(model, "3,99"), TextPosition.at(2, 3));
	assertLocation(parseStanzaGotoLocation(model, "0:0"), TextPosition.at(0, 0));
});

test("Go to Line supports backwards line/column values and UTF-16 offsets", () => {
	using model = new TextModel("alpha\n😊beta\ngamma");
	assertLocation(parseStanzaGotoLocation(model, "-1:-1"), TextPosition.at(2, 5));
	assertLocation(parseStanzaGotoLocation(model, "::8"), TextPosition.at(1, 1));
	assertLocation(parseStanzaGotoLocation(model, "::-1"), TextPosition.at(2, 4));
});

test("Go to Line reports incomplete and invalid input without changing the model", () => {
	using model = new TextModel("alpha");
	assert.equal(parseStanzaGotoLocation(model, "").kind, "empty");
	assert.equal(parseStanzaGotoLocation(model, "line").kind, "invalid");
	assert.equal(parseStanzaGotoLocation(model, "1:").kind, "invalid");
	assert.equal(parseStanzaGotoLocation(model, "::offset").kind, "invalid");
	assert.equal(model.getText(), "alpha");
});

function assertLocation(result: ReturnType<typeof parseStanzaGotoLocation>, position: TextPosition): void {
	assert.equal(result.kind, "location");
	if (result.kind === "location") assert.deepEqual(result.location.position, position);
}
