import assert from "node:assert/strict";
import test from "node:test";
import { DecorationPresentation, createStanzaDecorationRectangles, createStanzaDecorationSource } from "../../browser/viewParts/decorations/decorations.js";
import { type TextMeasurer } from "../../browser/config/fontMeasurements.js";
import { TextDecorationCollection } from "../../common/model/decorationCollection.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";
import { TextModel } from "../../common/model/textModel.js";
import { TrackedRangeStickiness, GlyphMarginLane } from '../../common/model.js';


test("Decoration source resolves opaque metadata without owning the collection", () => {
	using model = new TextModel("abcd\nefgh\nij");
	using collection = new TextDecorationCollection<DecorationMetadata>(model);
	const matchId = collection.add({
		range: Range.fromPositions(new Position((0) + 1, (1) + 1), new Position((1) + 1, (2) + 1)),
		stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
		metadata: { presentation: DecorationPresentation.SearchMatch },
	});
	collection.add({
		range: Range.fromPositions(new Position((1) + 1, (0) + 1), new Position((1) + 1, (1) + 1)),
		stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
		metadata: {},
	});
	const errorId = collection.add({
		range: Range.fromPositions(new Position((2) + 1, (0) + 1), new Position((2) + 1, (2) + 1)),
		stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
		metadata: { presentation: DecorationPresentation.ErrorUnderline },
	});
	const source = createStanzaDecorationSource(
		collection,
		decoration => decoration.metadata.presentation,
	);

	assert.deepEqual(source.decorations, [{
		id: matchId,
		range: Range.fromPositions(new Position((0) + 1, (1) + 1), new Position((1) + 1, (2) + 1)),
		presentation: DecorationPresentation.SearchMatch,
	}, {
		id: errorId,
		range: Range.fromPositions(new Position((2) + 1, (0) + 1), new Position((2) + 1, (2) + 1)),
		presentation: DecorationPresentation.ErrorUnderline,
	}]);
	assert.equal(Object.isFrozen(source.decorations), true);

	const rectangles = createStanzaDecorationRectangles(
		model,
		source.decorations,
		{ startLineIndex: 0, endLineIndexExclusive: 3 },
		38,
		new FixedTextMeasurer(),
	);
	assert.deepEqual(rectangles, [{
		id: matchId,
		presentation: DecorationPresentation.SearchMatch,
		lineIndex: 0,
		left: 48,
		width: 40,
	}, {
		id: matchId,
		presentation: DecorationPresentation.SearchMatch,
		lineIndex: 1,
		left: 38,
		width: 20,
	}, {
		id: errorId,
		presentation: DecorationPresentation.ErrorUnderline,
		lineIndex: 2,
		left: 38,
		width: 20,
	}]);

	let changes = 0;
	using listener = source.onDidChange(() => changes += 1);
	collection.delete(errorId);
	assert.equal(changes, 1);
	assert.equal(collection.size, 2);
});

test("Decoration geometry clips lines and rejects unknown presentations", () => {
	using model = new TextModel("abcd\nefgh");
	using collection = new TextDecorationCollection<string>(model);
	const id = collection.add({
		range: Range.fromPositions(new Position((0) + 1, (1) + 1), new Position((1) + 1, (2) + 1)),
		stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
		metadata: "match",
	});
	const source = createStanzaDecorationSource(
		collection,
		() => DecorationPresentation.SearchMatch,
	);

	assert.deepEqual(createStanzaDecorationRectangles(
		model,
		source.decorations,
		{ startLineIndex: 1, endLineIndexExclusive: 2 },
		38,
		new FixedTextMeasurer(),
	), [{
		id,
		presentation: DecorationPresentation.SearchMatch,
		lineIndex: 1,
		left: 38,
		width: 20,
	}]);

	const invalid = createStanzaDecorationSource(
		collection,
		() => "unknown" as DecorationPresentation,
	);
	assert.throws(() => invalid.decorations, /Unknown Stanza decoration/);
});

test("Decoration geometry presents an empty diagnostic at its text position", () => {
	using model = new TextModel("abcd\nefgh");
	using collection = new TextDecorationCollection<DecorationPresentation>(model);
	const id = collection.add({
		range: Range.fromPositions(new Position((1) + 1, (2) + 1)),
		stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
		metadata: DecorationPresentation.HintUnderline,
	});
	const source = createStanzaDecorationSource(collection, decoration => decoration.metadata);

	assert.deepEqual(createStanzaDecorationRectangles(
		model,
		source.decorations,
		{ startLineIndex: 0, endLineIndexExclusive: 2 },
		38,
		new FixedTextMeasurer(),
	), [{
		id,
		presentation: DecorationPresentation.HintUnderline,
		lineIndex: 1,
		left: 58,
		width: 10,
	}]);
});

test("Decoration sources declare and validate glyph-margin ownership", () => {
	using model = new TextModel("abc");
	using collection = new TextDecorationCollection<string>(model);
	const id = collection.add({
		range: Range.fromPositions(new Position((0) + 1, (0) + 1)),
		stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
		metadata: "folding",
	});
	const source = createStanzaDecorationSource(collection, () => ({
		presentation: DecorationPresentation.GlyphMargin,
		glyphMargin: { owner: "folding", lane: GlyphMarginLane.Center, ariaLabel: "Collapse lines", expanded: true },
	}), undefined, {
		glyphMarginLanes: [{ owner: "folding", lane: GlyphMarginLane.Center }],
	});

	assert.deepEqual(source.glyphMarginLanes, [{ owner: "folding", lane: GlyphMarginLane.Center }]);
	assert.deepEqual(source.decorations, [{
		id,
		range: Range.fromPositions(new Position((0) + 1, (0) + 1)),
		presentation: DecorationPresentation.GlyphMargin,
		glyphMargin: { owner: "folding", lane: GlyphMarginLane.Center, ariaLabel: "Collapse lines", expanded: true },
	}]);

	const undeclared = createStanzaDecorationSource(collection, () => ({
		presentation: DecorationPresentation.GlyphMargin,
		glyphMargin: { owner: "debug", lane: GlyphMarginLane.Left, ariaLabel: "Add breakpoint" },
	}));
	assert.throws(() => undeclared.decorations, /did not declare lane/);
});

test("Decoration sources declare and validate line-decoration ownership", () => {
	using model = new TextModel("abc");
	using collection = new TextDecorationCollection<string>(model);
	const id = collection.add({
		range: Range.fromPositions(new Position((0) + 1, (0) + 1)),
		stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
		metadata: "folding",
	});
	const source = createStanzaDecorationSource(collection, () => ({
		presentation: DecorationPresentation.LineDecoration,
		linesDecoration: { owner: "folding", className: "folding-marker" },
	}), undefined, {
		linesDecorationLanes: [{ owner: "folding", width: 20 }],
	});

	assert.deepEqual(source.linesDecorationLanes, [{ owner: "folding", width: 20 }]);
	assert.deepEqual(source.decorations, [{
		id,
		range: Range.fromPositions(new Position((0) + 1, (0) + 1)),
		presentation: DecorationPresentation.LineDecoration,
		linesDecoration: { owner: "folding", className: "folding-marker" },
	}]);

	const undeclared = createStanzaDecorationSource(collection, () => ({
		presentation: DecorationPresentation.LineDecoration,
		linesDecoration: { owner: "quick-diff", className: "quick-diff-marker" },
	}));
	assert.throws(() => undeclared.decorations, /did not declare a lane/);
});

interface DecorationMetadata {
	readonly presentation?: DecorationPresentation;
}

class FixedTextMeasurer implements TextMeasurer {
	readonly horizontalPadding = 24;
	readonly contentLeftPadding = 12;

	refresh(): boolean {
		return false;
	}

	measureLineWidth(text: string): number {
		return [...text].length * 10;
	}
}
