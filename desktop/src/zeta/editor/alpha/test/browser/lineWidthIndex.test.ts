import assert from "node:assert/strict";
import test from "node:test";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { AlphaLineWidthIndex } from "../../browser/lineWidthIndex.js";
import { TextRange } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

test("AlphaLineWidthIndex matches full scans across random transactions", () => {
  const random = seededRandom(0xA17A);
  const measurer = new WeightedTextMeasurer();
  using model = new TextModel(Array.from(
    { length: 30 },
    (_, index) => `initial ${index}`,
  ).join("\n"));
  const index = new AlphaLineWidthIndex(model, measurer);
  using listener = model.onDidChange(change => {
    index.applyModelChange(change);
  });

  for (let iteration = 0; iteration < 400; iteration++) {
    const action = random();
    if (action < 0.08 && model.canUndo) {
      model.undo();
    } else if (action < 0.12 && model.canRedo) {
      model.redo();
    } else {
      applyRandomTransaction(model, random);
    }
    assert.equal(
      index.maximumLineWidth,
      fullScanMaximum(model, measurer),
      `maximum width diverged at iteration ${iteration}`,
    );
  }
});

class WeightedTextMeasurer implements AlphaTextMeasurer {
  readonly horizontalPadding = 0;
  readonly contentLeftPadding = 0;

  refresh(): boolean {
    return false;
  }

  measureLineWidth(text: string): number {
    let width = 0;
    for (const character of text) {
      width += character === "\t"
        ? 11
        : character.codePointAt(0)! % 7 + 1;
    }
    return width;
  }
}

function fullScanMaximum(
  model: TextModel,
  measurer: AlphaTextMeasurer,
): number {
  let maximum = 0;
  for (let lineIndex = 0; lineIndex < model.lineCount; lineIndex++) {
    maximum = Math.max(
      maximum,
      measurer.measureLineWidth(model.getLineContent(lineIndex)),
    );
  }
  return maximum;
}

function applyRandomTransaction(
  model: TextModel,
  random: () => number,
): void {
  const length = model.createSnapshot().length;
  const edits = random() < 0.35 && length >= 4
    ? [
      randomEdit(model, random, 0, Math.floor(length / 2)),
      randomEdit(
        model,
        random,
        Math.floor(length / 2) + 1,
        length,
      ),
    ]
    : [randomEdit(model, random, 0, length)];
  model.applyEdits(edits);
}

function randomEdit(
  model: TextModel,
  random: () => number,
  minimumOffset: number,
  maximumOffset: number,
): {
  readonly range: TextRange;
  readonly text: string;
} {
  const first = randomInteger(
    random,
    minimumOffset,
    maximumOffset,
  );
  const second = randomInteger(
    random,
    minimumOffset,
    maximumOffset,
  );
  const startOffset = Math.min(first, second);
  const endOffset = Math.max(first, second);
  return {
    range: TextRange.from(
      model.positionAt(startOffset),
      model.positionAt(endOffset),
    ),
    text: randomText(random),
  };
}

function randomText(random: () => number): string {
  const alphabet = "abcXYZ09 \t\n";
  const length = randomInteger(random, 0, 12);
  let result = "";
  for (let index = 0; index < length; index++) {
    result += alphabet[randomInteger(random, 0, alphabet.length - 1)];
  }
  return result;
}

function randomInteger(
  random: () => number,
  minimum: number,
  maximum: number,
): number {
  return minimum + Math.floor(random() * (maximum - minimum + 1));
}

function seededRandom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1_664_525) + 1_013_904_223) >>> 0;
    return state / 0x1_0000_0000;
  };
}
