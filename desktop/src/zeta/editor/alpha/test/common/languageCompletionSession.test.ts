import { strict as assert } from "node:assert";
import test from "node:test";
import { EditorSelectionController } from "../../common/editorSelectionController.js";
import { LanguageCompletionDetailsStatus, LanguageCompletionSessionChangeReason, LanguageCompletionSessionController } from "../../common/languageCompletionSession.js";
import { LanguageResultAcceptance } from "../../common/languageResultStore.js";
import { LanguageCompletionItemKind, createLanguageCompletionStore, type LanguageCompletionItem, type LanguageCompletionItemDetails, type LanguageCompletionItemResolver, type LanguageCompletionResolveRequest } from "../../common/languageCompletions.js";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition, TextRange } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

test("Completion session opens at the matching cursor and navigates cyclically", () => {
  using model = new TextModel("con");
  using selections = controllerAt(model, TextPosition.at(0, 3));
  using store = createLanguageCompletionStore(model);
  using session = new LanguageCompletionSessionController(store, selections);
  const events: unknown[] = [];
  using listener = session.onDidChange(event => events.push({
    reason: event.reason,
    selected: event.state?.selectedItem.id,
  }));
  accept(store, model, 1, [
    completion("constant", "const"),
    completion("console", "console", true),
    completion("continue", "continue"),
  ]);

  assert.equal(session.state!.selectedItem.id, "console");
  assert.equal(session.selectNext(), true);
  assert.equal(session.state!.selectedItem.id, "continue");
  assert.equal(session.selectNext(), true);
  assert.equal(session.state!.selectedItem.id, "constant");
  assert.equal(session.selectPrevious(), true);
  assert.equal(session.state!.selectedItem.id, "continue");
  assert.deepEqual(events.map(event => (event as { reason: string }).reason), [
    LanguageCompletionSessionChangeReason.Store,
    LanguageCompletionSessionChangeReason.Focus,
    LanguageCompletionSessionChangeReason.Focus,
    LanguageCompletionSessionChangeReason.Focus,
  ]);
});

test("Same-version completion refresh retains focused item identity", () => {
  using model = new TextModel("con");
  using selections = controllerAt(model, TextPosition.at(0, 3));
  using store = createLanguageCompletionStore(model);
  using session = new LanguageCompletionSessionController(store, selections);
  accept(store, model, 1, [
    completion("one", "one"),
    completion("two", "two"),
  ]);
  session.selectIndex(1);

  accept(store, model, 2, [
    completion("two", "two updated"),
    completion("three", "three", true),
  ]);

  assert.equal(session.state!.requestId, 2);
  assert.equal(session.state!.selectedIndex, 0);
  assert.equal(session.state!.selectedItem.id, "two");
  assert.equal(session.state!.selectedItem.label, "two updated");
});

test("Accepting a completion is one isolated selection-aware undo step", () => {
  using model = new TextModel("con tail");
  using selections = controllerAt(model, TextPosition.at(0, 3));
  using store = createLanguageCompletionStore(model);
  using session = new LanguageCompletionSessionController(store, selections);
  const reasons: LanguageCompletionSessionChangeReason[] = [];
  using listener = session.onDidChange(event => reasons.push(event.reason));
  accept(store, model, 1, [
    completion("console", "console", false, TextRange.from(
      TextPosition.at(0, 0),
      TextPosition.at(0, 3),
    )),
  ]);

  assert.equal(session.acceptSelected(), true);
  assert.equal(model.getText(), "console tail");
  assert.equal(selections.selections.primary.active.compareTo(TextPosition.at(0, 7)), 0);
  assert.equal(session.state, undefined);
  assert.equal(reasons.at(-1), LanguageCompletionSessionChangeReason.Accepted);

  selections.undo();
  assert.equal(model.getText(), "con tail");
  assert.equal(selections.selections.primary.active.compareTo(TextPosition.at(0, 3)), 0);
});

test("Selection changes and explicit cancellation close only the local session", () => {
  using model = new TextModel("con");
  using selections = controllerAt(model, TextPosition.at(0, 3));
  using store = createLanguageCompletionStore(model);
  using session = new LanguageCompletionSessionController(store, selections);
  accept(store, model, 1, [completion("const", "const")]);

  selections.setSelections(TextSelectionSet.single(
    TextSelection.collapsedAt(TextPosition.at(0, 2)),
  ));
  assert.equal(session.state, undefined);
  assert.notEqual(store.result, undefined);

  selections.setSelections(TextSelectionSet.single(
    TextSelection.collapsedAt(TextPosition.at(0, 3)),
  ));
  accept(store, model, 2, [completion("continue", "continue")]);
  assert.equal(session.cancel(), true);
  assert.equal(session.cancel(), false);
  assert.notEqual(store.result, undefined);
});

test("Completion session rejects cross-model wiring and owns no dependencies", () => {
  using model = new TextModel("con");
  using otherModel = new TextModel("other");
  using selections = controllerAt(model, TextPosition.at(0, 3));
  using otherStore = createLanguageCompletionStore(otherModel);
  assert.throws(
    () => new LanguageCompletionSessionController(otherStore, selections),
    /must share one text model/,
  );

  using store = createLanguageCompletionStore(model);
  const session = new LanguageCompletionSessionController(store, selections);
  session.dispose();
  assert.throws(() => session.state, /already disposed/);
  accept(store, model, 1, [completion("const", "const")]);
  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 3)),
    text: "!",
  }]);
  assert.equal(model.getText(), "con!");
});

test("Completion session resolves only the focused item and cancels superseded details", async () => {
  using model = new TextModel("con");
  using selections = controllerAt(model, TextPosition.at(0, 3));
  using store = createLanguageCompletionStore(model);
  const resolver = new ControlledResolver();
  const errors: unknown[] = [];
  using session = new LanguageCompletionSessionController(store, selections, {
    resolver,
    onResolveError: error => errors.push(error),
  });
  accept(store, model, 1, [
    { ...completion("one", "one"), hasDeferredDetails: true },
    { ...completion("two", "two"), hasDeferredDetails: true },
  ]);
  await turn();

  assert.equal(session.state!.detailsStatus, LanguageCompletionDetailsStatus.Loading);
  assert.deepEqual(resolver.requests.map(entry => entry.request.itemId), ["one"]);
  session.selectIndex(1);
  await turn();
  assert.equal(resolver.requests[0]!.signal.aborted, true);
  assert.deepEqual(resolver.requests.map(entry => entry.request.itemId), ["one", "two"]);

  resolver.complete("two", {
    detail: "resolved detail",
    documentation: "resolved documentation",
  });
  await turn();

  assert.equal(session.state!.detailsStatus, LanguageCompletionDetailsStatus.Complete);
  assert.deepEqual(session.state!.details, {
    detail: "resolved detail",
    documentation: "resolved documentation",
  });
  assert.deepEqual(errors, []);

  accept(store, model, 2, [
    { ...completion("failed", "failed"), hasDeferredDetails: true },
  ]);
  await turn();
  resolver.fail("failed", new Error("resolve failed"));
  await turn();
  assert.equal(session.state!.detailsStatus, LanguageCompletionDetailsStatus.Failed);
  assert.match((errors[0] as Error).message, /resolve failed/);
});

function accept(
  store: ReturnType<typeof createLanguageCompletionStore>,
  model: TextModel,
  requestId: number,
  items: readonly LanguageCompletionItem[],
): void {
  assert.equal(store.accept({
    requestId,
    textModel: model,
    modelVersion: model.version,
    value: {
      position: TextPosition.at(0, 3),
      items,
      isIncomplete: false,
    },
  }), LanguageResultAcceptance.Applied);
}

function completion(id: string, label: string, preselect = false, range = TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 3))): LanguageCompletionItem {
  return {
    providerId: "test",
    id,
    label,
    kind: LanguageCompletionItemKind.Keyword,
    range,
    insertText: label,
    ...(preselect ? { preselect } : {}),
  };
}

function controllerAt(model: TextModel, position: TextPosition): EditorSelectionController {
  return new EditorSelectionController(
    model,
    TextSelectionSet.single(TextSelection.collapsedAt(position)),
  );
}

function turn(): Promise<void> {
  return new Promise(resolve => setImmediate(resolve));
}

class ControlledResolver implements LanguageCompletionItemResolver {
  readonly requests: Array<{
    readonly request: LanguageCompletionResolveRequest;
    readonly signal: AbortSignal;
    readonly resolve: (details: LanguageCompletionItemDetails) => void;
    readonly reject: (error: unknown) => void;
  }> = [];

  resolveCompletionItem(request: LanguageCompletionResolveRequest, signal: AbortSignal): Promise<LanguageCompletionItemDetails> {
    return new Promise((resolve, reject) => {
      this.requests.push({ request, signal, resolve, reject });
      signal.addEventListener("abort", () => reject(new Error("cancelled")), { once: true });
    });
  }

  complete(itemId: string, details: LanguageCompletionItemDetails): void {
    this.pending(itemId).resolve(details);
  }

  fail(itemId: string, error: Error): void {
    this.pending(itemId).reject(error);
  }

  private pending(itemId: string): (typeof this.requests)[number] {
    const request = this.requests.find(entry => entry.request.itemId === itemId && !entry.signal.aborted);
    assert.ok(request);
    return request;
  }
}
