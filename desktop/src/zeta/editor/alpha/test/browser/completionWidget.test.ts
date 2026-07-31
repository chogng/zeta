import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { EditorSelectionController } from "../../common/editorSelectionController.js";
import { LanguageCompletionDetailsStatus, LanguageCompletionSessionController, type LanguageCompletionSessionOptions } from "../../common/languageCompletionSession.js";
import { LanguageResultAcceptance } from "../../common/languageResultStore.js";
import { LanguageCompletionItemKind, createLanguageCompletionStore, type LanguageCompletionItem } from "../../common/languageCompletions.js";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition, TextRange } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
  MouseEvent: browserEnvironment.window.MouseEvent,
  KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { AlphaEditorViewport } = await import("../../browser/alphaEditorViewport.js");
const { AlphaTextInputController } = await import("../../browser/textInputController.js");

test("Completion widget projects named options, focus, ARIA, and content coordinates", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement<HTMLElement>(dom.window.document, "main");
  using model = new TextModel("con");
  using selections = controllerAt(model, TextPosition.at(0, 3));
  using store = createLanguageCompletionStore(model);
  using session = new LanguageCompletionSessionController(store, selections);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 300, height: 40 });
  using input = new AlphaTextInputController(viewport, selections, {
    completion: { session },
  });
  input.focus();
  accept(store, model, 1, [
    completion("constant", "const", LanguageCompletionItemKind.Keyword, "declaration"),
    completion("console", "console", LanguageCompletionItemKind.Variable, "global", true),
  ]);
  const widget = input.completionWidget!;
  const options = [...widget.element.querySelectorAll<HTMLElement>(".zeta-alpha-editor-completion-option")];

  assert.equal(widget.visible, true);
  assert.equal(widget.element.hidden, false);
  assert.equal(widget.element.style.left, "68px");
  assert.equal(widget.element.style.top, "20px");
  assert.equal(input.element.getAttribute("aria-autocomplete"), "list");
  assert.equal(input.element.getAttribute("aria-expanded"), "true");
  assert.equal(input.element.getAttribute("aria-controls"), widget.element.id);
  assert.equal(input.element.getAttribute("aria-activedescendant"), options[1]!.id);
  assert.deepEqual(options.map(option => ({
    selected: option.getAttribute("aria-selected"),
    focused: option.classList.contains("focused"),
    text: option.textContent,
  })), [{
    selected: "false",
    focused: false,
    text: "Keywordconstdeclaration",
  }, {
    selected: "true",
    focused: true,
    text: "Variableconsoleglobal",
  }]);
  dom.window.close();
});

test("Completion keyboard navigation accepts one item before ordinary input routing", () => {
  const fixture = createFixture("con");
  using resources = fixture;
  accept(fixture.store, fixture.model, 1, [
    completion("constant", "const", LanguageCompletionItemKind.Keyword),
    completion("console", "console", LanguageCompletionItemKind.Variable),
  ]);
  const down = keyboardEvent(fixture.dom.window, "ArrowDown");
  fixture.input.element.dispatchEvent(down);
  assert.equal(down.defaultPrevented, true);
  assert.equal(fixture.session.state!.selectedItem.id, "console");

  const enter = keyboardEvent(fixture.dom.window, "Enter");
  fixture.input.element.dispatchEvent(enter);
  assert.equal(enter.defaultPrevented, true);
  assert.equal(fixture.model.getText(), "console");
  assert.equal(fixture.selections.selections.primary.active.compareTo(TextPosition.at(0, 7)), 0);
  assert.equal(fixture.input.completionWidget!.visible, false);
  assert.equal(fixture.input.element.getAttribute("aria-expanded"), "false");
  assert.equal(fixture.dom.window.document.activeElement, fixture.input.element);

  fixture.selections.undo();
  assert.equal(fixture.model.getText(), "con");
});

test("Escape cancels locally while clicking accepts the selected option", () => {
  const fixture = createFixture("con");
  using resources = fixture;
  accept(fixture.store, fixture.model, 1, [
    completion("constant", "const", LanguageCompletionItemKind.Keyword),
  ]);
  const escape = keyboardEvent(fixture.dom.window, "Escape");
  fixture.input.element.dispatchEvent(escape);
  assert.equal(escape.defaultPrevented, true);
  assert.equal(fixture.input.completionWidget!.visible, false);
  assert.notEqual(fixture.store.result, undefined);

  accept(fixture.store, fixture.model, 2, [
    completion("constant", "const", LanguageCompletionItemKind.Keyword),
    completion("continue", "continue", LanguageCompletionItemKind.Keyword),
  ]);
  const option = requiredElement<HTMLElement>(
    fixture.input.completionWidget!.element,
    '[data-completion-index="1"]',
  );
  option.dispatchEvent(mouseEvent(fixture.dom.window, "mousedown"));
  option.dispatchEvent(mouseEvent(fixture.dom.window, "click"));
  assert.equal(fixture.model.getText(), "continue");
  assert.equal(fixture.input.completionWidget!.visible, false);
});

test("Completion widget validates ownership and restores input ARIA on disposal", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement<HTMLElement>(dom.window.document, "main");
  using model = new TextModel("con");
  using otherModel = new TextModel("other");
  using selections = controllerAt(model, TextPosition.at(0, 3));
  using otherSelections = controllerAt(otherModel, TextPosition.at(0, 5));
  using store = createLanguageCompletionStore(model);
  using otherStore = createLanguageCompletionStore(otherModel);
  using session = new LanguageCompletionSessionController(store, selections);
  using otherSession = new LanguageCompletionSessionController(otherStore, otherSelections);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  assert.throws(() => new AlphaTextInputController(viewport, selections, {
    completion: { session: otherSession },
  }), /must share one text model/);

  const input = new AlphaTextInputController(viewport, selections, {
    completion: { session },
  });
  assert.equal(input.element.getAttribute("aria-autocomplete"), "list");
  input.dispose();
  assert.equal(input.element.getAttribute("aria-autocomplete"), null);
  assert.equal(input.element.getAttribute("aria-controls"), null);

  accept(store, model, 1, [
    completion("constant", "const", LanguageCompletionItemKind.Keyword),
  ]);
  assert.notEqual(session.state, undefined);
  dom.window.close();
});

test("Disposing the common session immediately hides a surviving widget", () => {
  const fixture = createFixture("con");
  using resources = fixture;
  accept(fixture.store, fixture.model, 1, [
    completion("constant", "const", LanguageCompletionItemKind.Keyword),
  ]);
  assert.equal(fixture.input.completionWidget!.visible, true);

  fixture.session.dispose();

  assert.equal(fixture.input.completionWidget!.visible, false);
  assert.equal(fixture.input.element.getAttribute("aria-expanded"), "false");
  const down = keyboardEvent(fixture.dom.window, "ArrowDown");
  fixture.input.element.dispatchEvent(down);
  assert.equal(down.defaultPrevented, false);
});

test("Completion widget projects resolved details only for the focused option", async () => {
  const requests: string[] = [];
  const fixture = createFixture("con", {
    resolver: {
      resolveCompletionItem: async request => {
        requests.push(request.itemId);
        return {
          detail: "resolved detail",
          documentation: "Resolved documentation",
        };
      },
    },
  });
  using resources = fixture;
  accept(fixture.store, fixture.model, 1, [
    { ...completion("console", "console", LanguageCompletionItemKind.Variable), hasDeferredDetails: true },
    { ...completion("constant", "const", LanguageCompletionItemKind.Keyword), hasDeferredDetails: true },
  ]);
  await new Promise<void>(resolve => setImmediate(resolve));
  await new Promise<void>(resolve => setImmediate(resolve));
  const selected = requiredElement<HTMLElement>(
    fixture.input.completionWidget!.element,
    '[data-completion-index="0"]',
  );

  assert.deepEqual(requests, ["console"]);
  assert.equal(fixture.session.state!.detailsStatus, LanguageCompletionDetailsStatus.Complete);
  assert.equal(selected.querySelector(".zeta-alpha-editor-completion-detail")!.textContent, "resolved detail");
  assert.equal(selected.querySelector(".zeta-alpha-editor-completion-documentation")!.textContent, "Resolved documentation");
  assert.equal(selected.classList.contains("resolving"), false);
  assert.equal(selected.getAttribute("aria-busy"), null);
});

interface CompletionFixture extends Disposable {
  readonly dom: JSDOM;
  readonly model: TextModel;
  readonly selections: EditorSelectionController;
  readonly store: ReturnType<typeof createLanguageCompletionStore>;
  readonly session: LanguageCompletionSessionController;
  readonly viewport: InstanceType<typeof AlphaEditorViewport>;
  readonly input: InstanceType<typeof AlphaTextInputController>;
}

function createFixture(text: string, sessionOptions: LanguageCompletionSessionOptions = {}): CompletionFixture {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const model = new TextModel(text);
  const selections = controllerAt(model, TextPosition.at(0, text.length));
  const store = createLanguageCompletionStore(model);
  const session = new LanguageCompletionSessionController(store, selections, sessionOptions);
  const viewport = new AlphaEditorViewport({
    container: requiredElement<HTMLElement>(dom.window.document, "main"),
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 300, height: 40 });
  const input = new AlphaTextInputController(viewport, selections, {
    completion: { session },
  });
  input.focus();
  return {
    dom,
    model,
    selections,
    store,
    session,
    viewport,
    input,
    [Symbol.dispose](): void {
      input.dispose();
      viewport.dispose();
      session.dispose();
      store.dispose();
      selections.dispose();
      model.dispose();
      dom.window.close();
    },
  };
}

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

function completion(id: string, label: string, kind: LanguageCompletionItemKind, detail?: string, preselect = false): LanguageCompletionItem {
  return {
    providerId: "test",
    id,
    label,
    kind,
    range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 3)),
    insertText: label,
    ...(detail === undefined ? {} : { detail }),
    ...(preselect ? { preselect } : {}),
  };
}

function controllerAt(model: TextModel, position: TextPosition): EditorSelectionController {
  return new EditorSelectionController(
    model,
    TextSelectionSet.single(TextSelection.collapsedAt(position)),
  );
}

function keyboardEvent(targetWindow: typeof browserEnvironment.window, key: string): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key,
  }) as unknown as KeyboardEvent;
}

function mouseEvent(targetWindow: typeof browserEnvironment.window, type: string): MouseEvent {
  return new targetWindow.MouseEvent(type, {
    bubbles: true,
    cancelable: true,
    button: 0,
  }) as unknown as MouseEvent;
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
  const element = root.querySelector<T>(selector);
  assert.ok(element);
  return element;
}

class FixedTextMeasurer implements AlphaTextMeasurer {
  readonly horizontalPadding = 24;
  readonly contentLeftPadding = 12;

  refresh(): boolean {
    return false;
  }

  measureLineWidth(text: string): number {
    return text.length * 10;
  }
}
