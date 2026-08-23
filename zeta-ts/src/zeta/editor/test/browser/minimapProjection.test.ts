import assert from "node:assert/strict";
import test from "node:test";
import { createMinimapRows } from "../../browser/viewparts/minimap/minimapProjection.js";

test("Minimap compacts a document into bounded sampled density rows", () => {
	const lines = ["short", "", "very long source line", "middle", "", "tail"];
	const rows = createMinimapRows(source(lines), 3);

	assert.deepEqual(rows, [
		{ startLineIndex: 0, endLineIndexExclusive: 2, density: 5 / 21 },
		{ startLineIndex: 2, endLineIndexExclusive: 4, density: 1 },
		{ startLineIndex: 4, endLineIndexExclusive: 6, density: 4 / 21 },
	]);
});

test("Minimap omits wholly blank document regions and validates its inputs", () => {
	assert.deepEqual(createMinimapRows(source(["", "  ", "code", ""]), 4), [
		{ startLineIndex: 2, endLineIndexExclusive: 3, density: 1 },
	]);
	assert.throws(() => createMinimapRows(source(["one"]), 0), /positive safe integer/);
	assert.throws(() => createMinimapRows({ lineCount: 0, getLineContent: () => "" }), /non-empty text source/);
});

function source(lines: readonly string[]) {
	return {
		lineCount: lines.length,
		getLineContent: (lineIndex: number): string => lines[lineIndex]!,
	};
}
