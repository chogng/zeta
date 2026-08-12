import assert from "node:assert/strict";
import test from "node:test";
import { DecorationPresentation, createAlphaDecorationRectangles, createAlphaDecorationSource } from "../../browser/view/decorationPresentation.js";
import { type TextMeasurer } from "../../browser/view/fontMetrics.js";
import { TextDecorationCollection } from "../../common/model/decorationCollection.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { TrackedRangeStickiness } from "../../common/model/trackedRange.js";

test("Decoration source resolves opaque metadata without owning the collection", () => {
  using model = new TextModel("abcd\nefgh\nij");
  using collection = new TextDecorationCollection<DecorationMetadata>(model);
  const matchId = collection.add({
    range: TextRange.from(TextPosition.at(0, 1), TextPosition.at(1, 2)),
    stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
    metadata: { presentation: DecorationPresentation.SearchMatch },
  });
  collection.add({
    range: TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 1)),
    stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
    metadata: {},
  });
  const errorId = collection.add({
    range: TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 2)),
    stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
    metadata: { presentation: DecorationPresentation.ErrorUnderline },
  });
  const source = createAlphaDecorationSource(
    collection,
    decoration => decoration.metadata.presentation,
  );

  assert.deepEqual(source.decorations, [{
    id: matchId,
    range: TextRange.from(TextPosition.at(0, 1), TextPosition.at(1, 2)),
    presentation: DecorationPresentation.SearchMatch,
  }, {
    id: errorId,
    range: TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 2)),
    presentation: DecorationPresentation.ErrorUnderline,
  }]);
  assert.equal(Object.isFrozen(source.decorations), true);

  const rectangles = createAlphaDecorationRectangles(
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
    range: TextRange.from(TextPosition.at(0, 1), TextPosition.at(1, 2)),
    stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
    metadata: "match",
  });
  const source = createAlphaDecorationSource(
    collection,
    () => DecorationPresentation.SearchMatch,
  );

  assert.deepEqual(createAlphaDecorationRectangles(
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

  const invalid = createAlphaDecorationSource(
    collection,
    () => "unknown" as DecorationPresentation,
  );
  assert.throws(() => invalid.decorations, /Unknown Alpha decoration/);
});

test("Decoration geometry presents an empty diagnostic at its text position", () => {
  using model = new TextModel("abcd\nefgh");
  using collection = new TextDecorationCollection<DecorationPresentation>(model);
  const id = collection.add({
    range: TextRange.emptyAt(TextPosition.at(1, 2)),
    stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
    metadata: DecorationPresentation.HintUnderline,
  });
  const source = createAlphaDecorationSource(collection, decoration => decoration.metadata);

  assert.deepEqual(createAlphaDecorationRectangles(
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
