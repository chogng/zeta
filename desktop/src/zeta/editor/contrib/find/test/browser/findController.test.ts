import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type TextMeasurer } from "../../../../browser/view/fontMetrics.js";
import { TextDecorationCollection } from "../../../../common/model/decorationCollection.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
  InputEvent: browserEnvironment.window.InputEvent,
  MouseEvent: browserEnvironment.window.MouseEvent,
  KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { EditorViewport } = await import("../../../../browser/view/editorViewport.js");
const { DecorationPresentation, createAlphaDecorationSource } = await import("../../../../browser/view/decorationPresentation.js");
const { FindController } = await import("../../browser/findController.js");

test.after(() => browserEnvironment.window.close());

test("find opens from the editor shortcut, highlights matches, navigates, and restores focus", () => {
  const fixture = createFixture("alpha beta alpha", TextPosition.at(0, 0), TextPosition.at(0, 5));
  using resources = fixture;

  const open = keyboardEvent(fixture.dom.window, "f", { ctrlKey: true });
  fixture.editorInput.dispatchEvent(open);

  assert.equal(open.defaultPrevented, true);
  assert.equal(fixture.find.visible, true);
  assert.equal(fixture.find.searchInput.value, "alpha");
  assert.equal(fixture.find.element.querySelector(".zeta-alpha-editor-find-result")?.textContent, "1 of 2");
  assert.equal(fixture.viewport.element.querySelectorAll(".zeta-alpha-editor-decoration.search-match").length, 2);
  assert.equal(fixture.dom.window.document.activeElement, fixture.find.searchInput);

  fixture.find.searchInput.dispatchEvent(keyboardEvent(fixture.dom.window, "Enter"));
  assert.deepEqual(fixture.selections.selections.primary.range.start, TextPosition.at(0, 11));
  assert.deepEqual(fixture.selections.selections.primary.range.end, TextPosition.at(0, 16));
  assert.equal(fixture.find.element.querySelector(".zeta-alpha-editor-find-result")?.textContent, "2 of 2");

  fixture.find.searchInput.dispatchEvent(keyboardEvent(fixture.dom.window, "Escape"));
  assert.equal(fixture.find.visible, false);
  assert.equal(fixture.decorations.size, 0);
  assert.equal(fixture.dom.window.document.activeElement, fixture.editorInput);
});

test("find options project checked classes and report invalid regular expressions", () => {
  const fixture = createFixture("Alpha alpha alphabet");
  using resources = fixture;
  fixture.find.open();
  setInputValue(fixture.find.searchInput, "alpha");

  const matchCase = requiredElement<HTMLButtonElement>(fixture.find.element, '[aria-label="Match case"]');
  const wholeWord = requiredElement<HTMLButtonElement>(fixture.find.element, '[aria-label="Match whole word"]');
  matchCase.click();
  wholeWord.click();
  assert.equal(matchCase.classList.contains("checked"), true);
  assert.equal(matchCase.getAttribute("aria-pressed"), "true");
  assert.equal(wholeWord.classList.contains("checked"), true);
  assert.equal(fixture.decorations.size, 1);

  const regularExpression = requiredElement<HTMLButtonElement>(fixture.find.element, '[aria-label="Use regular expression"]');
  regularExpression.click();
  setInputValue(fixture.find.searchInput, "(");
  assert.equal(regularExpression.classList.contains("checked"), true);
  assert.equal(fixture.find.searchInput.getAttribute("aria-invalid"), "true");
  assert.equal(fixture.find.element.querySelector(".zeta-alpha-editor-find-result")?.textContent, "Invalid expression");
  assert.equal(fixture.decorations.size, 0);
});

test("find in selection keeps the opening scope through match navigation and supports Alt+L", () => {
  const fixture = createFixture("alpha beta alpha beta alpha", TextPosition.at(0, 6), TextPosition.at(0, 16));
  using resources = fixture;
  fixture.find.open();
  setInputValue(fixture.find.searchInput, "alpha");

  const findInSelection = requiredElement<HTMLButtonElement>(fixture.find.element, '[aria-label="Find in selection"]');
  assert.equal(findInSelection.disabled, false);
  assert.equal(fixture.decorations.size, 3);
  findInSelection.click();

  assert.equal(findInSelection.classList.contains("checked"), true);
  assert.equal(findInSelection.getAttribute("aria-pressed"), "true");
  assert.equal(fixture.decorations.size, 1);
  assert.deepEqual(fixture.selections.selections.primary.range.start, TextPosition.at(0, 11));
  assert.deepEqual(fixture.selections.selections.primary.range.end, TextPosition.at(0, 16));

  const toggle = keyboardEvent(fixture.dom.window, "l", { altKey: true });
  fixture.find.searchInput.dispatchEvent(toggle);
  assert.equal(toggle.defaultPrevented, true);
  assert.equal(findInSelection.classList.contains("checked"), false);
  assert.equal(fixture.decorations.size, 3);
});

test("find in selection restricts replace all to its tracked opening scope", () => {
  const fixture = createFixture("alpha beta alpha beta alpha", TextPosition.at(0, 6), TextPosition.at(0, 16));
  using resources = fixture;
  fixture.find.open({ showReplace: true });
  setInputValue(fixture.find.searchInput, "alpha");
  requiredElement<HTMLButtonElement>(fixture.find.element, '[aria-label="Find in selection"]').click();
  fixture.find.replaceInput.value = "x";
  requiredElement<HTMLButtonElement>(fixture.find.element, '[aria-label="Replace all matches"]').click();

  assert.equal(fixture.model.getText(), "alpha beta x beta alpha");
  fixture.selections.undo();
  assert.equal(fixture.model.getText(), "alpha beta alpha beta alpha");
});

test("replace current and replace all use isolated undo transactions", () => {
  const fixture = createFixture("a a a");
  using resources = fixture;
  const openReplace = keyboardEvent(fixture.dom.window, "h", { ctrlKey: true });
  fixture.editorInput.dispatchEvent(openReplace);
  setInputValue(fixture.find.searchInput, "a");
  fixture.find.replaceInput.value = "long";

  assert.equal(openReplace.defaultPrevented, true);
  assert.equal(fixture.find.replaceInput.closest(".zeta-alpha-editor-replace-row")?.hasAttribute("hidden"), false);
  requiredElement<HTMLButtonElement>(fixture.find.element, '[aria-label="Replace current match"]').click();
  assert.equal(fixture.model.getText(), "long a a");

  fixture.find.replaceInput.value = "x";
  requiredElement<HTMLButtonElement>(fixture.find.element, '[aria-label="Replace all matches"]').click();
  assert.equal(fixture.model.getText(), "long x x");

  fixture.selections.undo();
  assert.equal(fixture.model.getText(), "long a a");
  fixture.selections.undo();
  assert.equal(fixture.model.getText(), "a a a");
});

interface Fixture extends Disposable {
  readonly dom: JSDOM;
  readonly model: TextModel;
  readonly selections: EditorSelectionController;
  readonly decorations: TextDecorationCollection<void>;
  readonly viewport: InstanceType<typeof EditorViewport>;
  readonly editorInput: HTMLTextAreaElement;
  readonly find: InstanceType<typeof FindController>;
}

function createFixture(text: string, anchor = TextPosition.at(0, 0), active = anchor): Fixture {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement<HTMLElement>(dom.window.document, "main");
  const model = new TextModel(text);
  const selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.from(anchor, active)));
  const decorations = new TextDecorationCollection<void>(model);
  const viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
    decorationSources: [createAlphaDecorationSource(decorations, () => DecorationPresentation.SearchMatch)],
  });
  viewport.layout({ width: 600, height: 120 });
  const editorInput = dom.window.document.createElement("textarea") as unknown as HTMLTextAreaElement;
  viewport.element.append(editorInput);
  const find = new FindController(editorInput, viewport, selections, decorations);
  return {
    dom,
    model,
    selections,
    decorations,
    viewport,
    editorInput,
    find,
    [Symbol.dispose](): void {
      find.dispose();
      editorInput.remove();
      viewport.dispose();
      decorations.dispose();
      selections.dispose();
      model.dispose();
      dom.window.close();
    },
  };
}

function setInputValue(input: HTMLInputElement, value: string): void {
  input.value = value;
  input.dispatchEvent(new input.ownerDocument.defaultView!.Event("input", { bubbles: true }));
}

function keyboardEvent(targetWindow: typeof browserEnvironment.window, key: string, options: KeyboardEventInit = {}): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key,
    ...options,
  }) as unknown as KeyboardEvent;
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
  const element = root.querySelector<T>(selector);
  assert.ok(element);
  return element;
}

class FixedTextMeasurer implements TextMeasurer {
  readonly horizontalPadding = 24;
  readonly contentLeftPadding = 12;

  refresh(): boolean {
    return false;
  }

  measureLineWidth(text: string): number {
    return text.length * 10;
  }
}
