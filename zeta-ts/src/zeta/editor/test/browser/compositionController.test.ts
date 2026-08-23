import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { IME } from "../../../base/common/ime.js";
import { type TextMeasurer } from "../../browser/measurement/fontMetrics.js";
import { EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

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

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
  InputEvent: browserEnvironment.window.InputEvent,
  KeyboardEvent: browserEnvironment.window.KeyboardEvent,
  CompositionEvent: browserEnvironment.window.CompositionEvent,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { EditorViewport } = await import("../../browser/view/editorViewport.js");
const { TextInputController } = await import("../../browser/input/textInputController.js");

test("Textarea composition commits one revision and positions the IME input", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("hello");
  const initial = TextSelectionSet.single(selection(0, 1, 0, 4));
  using selections = new EditorSelectionController(model, initial);
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 100, height: 20 });
  using input = new TextInputController(viewport, selections);
  const states: boolean[] = [];
  using listener = input.compositionController.onDidChange(state => states.push(state));

  input.element.dispatchEvent(compositionEvent(dom.window, "compositionstart", ""));
  assert.deepEqual({
    composing: input.compositionController.composing,
    rootClass: viewport.element.classList.contains("composing"),
    inputClass: input.element.classList.contains("ime-input"),
    left: input.element.style.left,
    top: input.element.style.top,
    height: input.element.style.height,
  }, {
    composing: true,
    rootClass: true,
    inputClass: true,
    left: "48px",
    top: "0px",
    height: "20px",
  });

  input.element.value = "ni";
  input.element.setSelectionRange(1, 2, "backward");
  input.element.dispatchEvent(compositionEvent(dom.window, "compositionupdate", "ni"));
  const firstUnderline = viewport.element.querySelector<HTMLElement>(
    ".aster-editor-composition",
  );
  assert.ok(firstUnderline);
  assert.deepEqual({
    text: model.getText(),
    selection: selections.selections.primary,
    underline: {
      left: firstUnderline.style.left,
      width: firstUnderline.style.width,
    },
  }, {
    text: "hnio",
    selection: TextSelection.from(TextPosition.at(0, 3), TextPosition.at(0, 2)),
    underline: {
      left: "48px",
      width: "20px",
    },
  });

  input.element.value = "你";
  input.element.setSelectionRange(1, 1);
  input.element.dispatchEvent(compositionEvent(dom.window, "compositionupdate", "你"));
  input.element.dispatchEvent(compositionEvent(dom.window, "compositionend", "你"));

  assert.deepEqual({
    text: model.getText(),
    selection: selections.selections.primary,
    composing: input.compositionController.composing,
    rootClass: viewport.element.classList.contains("composing"),
    inputValue: input.element.value,
    underlineCount: viewport.element.querySelectorAll(
      ".aster-editor-composition",
    ).length,
    states,
  }, {
    text: "h你o",
    selection: caret(0, 2),
    composing: false,
    rootClass: false,
    inputValue: "",
    underlineCount: 0,
    states: [true, false],
  });

  selections.undo();
  assert.deepEqual({
    text: model.getText(),
    selections: selections.selections,
  }, {
    text: "hello",
    selections: initial,
  });

  dom.window.close();
});

test("Escape, blur, and disposal cancel active textarea composition", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("abc");
  const initial = TextSelectionSet.single(caret(0, 1));
  using selections = new EditorSelectionController(model, initial);
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 100, height: 20 });
  const input = new TextInputController(viewport, selections);

  startAndUpdate(dom.window, input.element, "中");
  const positionedInput = {
    left: input.element.style.left,
    top: input.element.style.top,
    height: input.element.style.height,
  };
  input.element.dispatchEvent(keyboardEvent(dom.window, "Escape", true));
  input.element.dispatchEvent(compositionEvent(dom.window, "compositionend", "中"));
  assert.deepEqual({
    text: model.getText(),
    selections: selections.selections,
    canUndo: model.canUndo,
  }, {
    text: "abc",
    selections: initial,
    canUndo: false,
  });
  assert.deepEqual({
    left: input.element.style.left,
    top: input.element.style.top,
    height: input.element.style.height,
  }, { left: "", top: "", height: "" });

  input.element.focus();
  startAndUpdate(dom.window, input.element, "X");
  assert.deepEqual({
    left: input.element.style.left,
    top: input.element.style.top,
    height: input.element.style.height,
  }, positionedInput);
  input.element.blur();
  assert.equal(model.getText(), "abc");
  assert.equal(input.compositionController.composing, false);

  input.element.focus();
  startAndUpdate(dom.window, input.element, "Y");
  input.dispose();
  assert.equal(model.getText(), "abc");
  assert.equal(viewport.element.classList.contains("composing"), false);

  dom.window.close();
});

test("Empty composition end commits deletion while a stray end is ignored", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("abc");
  const initial = TextSelectionSet.single(selection(0, 1, 0, 2));
  using selections = new EditorSelectionController(model, initial);
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  using input = new TextInputController(viewport, selections);

  input.element.dispatchEvent(compositionEvent(dom.window, "compositionstart", ""));
  input.element.dispatchEvent(compositionEvent(dom.window, "compositionend", ""));
  input.element.dispatchEvent(compositionEvent(dom.window, "compositionend", ""));
  assert.deepEqual({
    text: model.getText(),
    selection: selections.selections.primary,
  }, {
    text: "ac",
    selection: caret(0, 1),
  });

  selections.undo();
  assert.deepEqual({
    text: model.getText(),
    selections: selections.selections,
  }, {
    text: "abc",
    selections: initial,
  });

  dom.window.close();
});

test("IME coordination, multi-cursor rejection, and external invalidation are safe", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("a\nbc");
  using selections = new EditorSelectionController(
    model,
    TextSelectionSet.withPrimary([caret(1, 1), caret(0, 0)], 0),
  );
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 100, height: 40 });
  using input = new TextInputController(viewport, selections);

  const multiStart = compositionEvent(dom.window, "compositionstart", "");
  input.element.dispatchEvent(multiStart);
  assert.equal(multiStart.defaultPrevented, true);
  assert.equal(input.compositionController.composing, false);

  selections.setSelections(TextSelectionSet.single(caret(1, 1)));
  try {
    IME.disable();
    assert.equal(input.element.readOnly, true);
    const disabledStart = compositionEvent(dom.window, "compositionstart", "");
    input.element.dispatchEvent(disabledStart);
    assert.equal(disabledStart.defaultPrevented, true);
    IME.enable();
    assert.equal(input.element.readOnly, false);

    input.element.dispatchEvent(compositionEvent(dom.window, "compositionstart", ""));
    input.element.value = "x\r\ny";
    input.element.setSelectionRange(4, 4);
    input.element.dispatchEvent(compositionEvent(
      dom.window,
      "compositionupdate",
      "x\r\ny",
    ));
    assert.deepEqual({
      text: model.getText(),
      selection: selections.selections.primary,
      top: input.element.style.top,
      underlines: [...viewport.element.querySelectorAll<HTMLElement>(
        ".aster-editor-composition",
      )].map(element => ({
        left: element.style.left,
        width: element.style.width,
      })),
    }, {
      text: "a\nbx\nyc",
      selection: caret(2, 1),
      top: "40px",
      underlines: [
        { left: "48px", width: "20px" },
        { left: "38px", width: "10px" },
      ],
    });

    model.applyEdits([{
      range: TextRange.emptyAt(model.positionAt(model.getText().length)),
      text: "!",
    }]);
    assert.equal(input.compositionController.composing, false);
    assert.equal(viewport.element.classList.contains("composing"), false);
    assert.equal(viewport.element.querySelectorAll(
      ".aster-editor-composition",
    ).length, 0);
    input.element.dispatchEvent(compositionEvent(dom.window, "compositionupdate", "ignored"));
    input.element.dispatchEvent(compositionEvent(dom.window, "compositionend", "ignored"));
    assert.equal(model.getText(), "a\nbx\nyc!");
  } finally {
    IME.enable();
  }

  dom.window.close();
});

function startAndUpdate(targetWindow: typeof browserEnvironment.window, element: HTMLTextAreaElement, text: string): void {
  element.dispatchEvent(compositionEvent(targetWindow, "compositionstart", ""));
  element.value = text;
  element.setSelectionRange(text.length, text.length);
  element.dispatchEvent(compositionEvent(targetWindow, "compositionupdate", text));
}

function compositionEvent(targetWindow: typeof browserEnvironment.window, type: "compositionstart" | "compositionupdate" | "compositionend", data: string): CompositionEvent {
  return new targetWindow.CompositionEvent(type, {
    bubbles: true,
    cancelable: true,
    data,
  }) as unknown as CompositionEvent;
}

function keyboardEvent(targetWindow: typeof browserEnvironment.window, key: string, isComposing: boolean): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key,
    isComposing,
  }) as unknown as KeyboardEvent;
}

function selection(startLine: number, startColumn: number, endLine: number, endColumn: number): TextSelection {
  return TextSelection.from(
    TextPosition.at(startLine, startColumn),
    TextPosition.at(endLine, endColumn),
  );
}

function caret(lineIndex: number, columnIndex: number): TextSelection {
  return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}
