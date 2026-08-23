import assert from "node:assert/strict";
import test from "node:test";
import { TextSelection } from "../../../../common/core/selection.js";
import { TextPosition, TextRange } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { expandSmartSelection } from "../../common/smartSelect.js";

test("Smart select expands a caret through word, pair, line, and document scopes", () => {
  using model = new TextModel("const value = (one + two);\nnext");
  const caret = TextSelection.collapsedAt(TextPosition.at(0, 16));
  const word = expandSmartSelection(model, caret);
  assert.equal(model.getTextInRange(word.range), "one");
  const pair = expandSmartSelection(model, word);
  assert.equal(model.getTextInRange(pair.range), "(one + two)");
  const line = expandSmartSelection(model, pair);
  assert.equal(model.getTextInRange(line.range), "const value = (one + two);");
});

test("Smart select prefers the smallest parser scope before lexical pair and line fallbacks", () => {
  using model = new TextModel("fn outer() { let value = call(1 + 2); }");
  const valueStart = model.getText().indexOf("value");
  const declarationStart = model.getText().indexOf("let value");
  const declarationEnd = model.getText().indexOf(";", declarationStart) + 1;
  const functionEnd = model.length;
  const syntaxRanges = [
    TextRange.from(model.positionAt(valueStart), model.positionAt(valueStart + "value".length)),
    TextRange.from(model.positionAt(declarationStart), model.positionAt(declarationEnd)),
    TextRange.from(model.positionAt(0), model.positionAt(functionEnd)),
  ];
  const caret = TextSelection.collapsedAt(model.positionAt(valueStart + 2));

  const word = expandSmartSelection(model, caret, undefined, syntaxRanges);
  const declaration = expandSmartSelection(model, word, undefined, syntaxRanges);
  const functionScope = expandSmartSelection(model, declaration, undefined, syntaxRanges);

  assert.equal(model.getTextInRange(word.range), "value");
  assert.equal(model.getTextInRange(declaration.range), "let value = call(1 + 2);");
  assert.equal(model.getTextInRange(functionScope.range), model.getText());
});
