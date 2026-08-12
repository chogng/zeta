import { strict as assert } from "node:assert";
import test from "node:test";
import { createLanguageCompletionInvokeContext, type LanguageCompletionProviderRequest } from "../../common/languages/completion/languageCompletionProviders.js";
import { createLanguageWordCompletionProvider } from "../../common/languages/completion/languageWordCompletionProvider.js";
import { TextPosition } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

test("Word completion is deterministic, bounded, and snapshot-local", async () => {
  using model = new TextModel("alpine alpha alphabet al");
  const provider = createLanguageWordCompletionProvider({ maximumItems: 2 });
  const position = TextPosition.at(0, model.getText().length);

  const result = await provider.provideCompletions(request(model, position), new AbortController().signal);

  assert.deepEqual(result?.items.map(item => item.label), ["alpha", "alphabet"]);
  assert.equal(result?.isIncomplete, true);
  assert.equal(result?.items.every(item => /^[A-Za-z0-9._-]+$/.test(item.id)), true);
  assert.equal(result?.items[0]!.range.start.columnIndex, position.columnIndex - 2);
  assert.deepEqual(result?.items[0]!.range.end, position);
});

test("Word completion replaces a complete active segment from a mid-word caret", async () => {
  using model = new TextModel("connection console");
  const provider = createLanguageWordCompletionProvider();
  const position = TextPosition.at(0, "connection con".length);

  const result = await provider.provideCompletions(request(model, position), new AbortController().signal);

  assert.deepEqual(result?.items.map(item => item.label), ["connection"]);
  assert.equal(result?.items[0]!.range.start.columnIndex, "connection ".length);
  assert.equal(result?.items[0]!.range.end.columnIndex, model.getText().length);
});

test("Word completion validates limits and observes cancellation", async () => {
  assert.throws(() => createLanguageWordCompletionProvider({ maximumItems: 0 }), /positive safe integer/);
  using model = new TextModel("alpha al");
  const provider = createLanguageWordCompletionProvider();
  const controller = new AbortController();
  controller.abort("cancelled");

  assert.throws(
    () => provider.provideCompletions(
      request(model, TextPosition.at(0, model.getText().length)),
      controller.signal,
    ),
  );
});

function request(model: TextModel, position: TextPosition): LanguageCompletionProviderRequest {
  return Object.freeze({
    requestId: 1,
    snapshot: model.createSnapshot(),
    languageId: "plaintext",
    position,
    context: createLanguageCompletionInvokeContext(),
  });
}
