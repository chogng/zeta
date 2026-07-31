import assert from "node:assert/strict";
import test from "node:test";
import { TextRange } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

test("TextModel replays deterministic history across changing line maps", () => {
  using model = new TextModel("seed\ntext");
  const states = [model.getText()];
  const random = createRandom(0x5e7a);

  for (let index = 0; index < 200; index += 1) {
    const length = model.getText().length;
    const startOffset = Math.floor(random() * (length + 1));
    const deleteLength = index % 3 === 0 && startOffset < length
      ? 1
      : 0;
    const insertedText = deleteLength > 0
      ? ""
      : index % 5 === 0
        ? `\r\n${index}`
        : String.fromCharCode(97 + index % 26);
    model.applyEdits([{
      range: TextRange.from(
        model.positionAt(startOffset),
        model.positionAt(startOffset + deleteLength),
      ),
      text: insertedText,
    }]);
    states.push(model.getText());
  }

  for (let index = states.length - 2; index >= 0; index -= 1) {
    model.undo();
    assert.equal(model.getText(), states[index]);
  }
  for (let index = 1; index < states.length; index += 1) {
    model.redo();
    assert.equal(model.getText(), states[index]);
  }
});

test("TextModel keeps transaction identity across undo and redo", () => {
  using model = new TextModel("abc");
  const first = model.applyEdits([{
    range: TextRange.emptyAt(model.positionAt(0)),
    text: "X",
  }]);
  const second = model.applyEdits([{
    range: TextRange.emptyAt(model.positionAt(2)),
    text: "Y",
  }]);
  const undoSecond = model.undo();
  const undoFirst = model.undo();
  const redoFirst = model.redo();

  assert.deepEqual({
    first: first?.transactionId,
    second: second?.transactionId,
    undoSecond: undoSecond?.transactionId,
    undoFirst: undoFirst?.transactionId,
    redoFirst: redoFirst?.transactionId,
  }, {
    first: 1,
    second: 2,
    undoSecond: 2,
    undoFirst: 1,
    redoFirst: 1,
  });
});

function createRandom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1_664_525) + 1_013_904_223) >>> 0;
    return state / 0x1_0000_0000;
  };
}
