import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { EditorSelectionController } from "../../common/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";

class FixedTextMeasurer implements AlphaTextMeasurer {
  readonly horizontalPadding = 24;
  readonly contentLeftPadding = 12;

  refresh(): boolean {
    return false;
  }

  measureLineWidth(text: string): number {
    return [...text].length * 10;
  }
}

class MemoryClipboardData {
  readonly files: readonly File[] = [];
  private readonly values = new Map<string, string>();

  get types(): readonly string[] {
    return [...this.values.keys()];
  }

  getData(type: string): string {
    return this.values.get(type) ?? "";
  }

  setData(type: string, value: string): void {
    this.values.set(type, value);
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
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { AlphaEditorViewport } = await import("../../browser/alphaEditorViewport.js");
const { ALPHA_EDITOR_CLIPBOARD_MIME, AlphaClipboardLineEnding } = await import("../../browser/clipboardController.js");
const { EditorClipboardPasteMode, EditorEmptySelectionClipboardPolicy } = await import("../../common/clipboard.js");
const { AlphaTextInputController } = await import("../../browser/textInputController.js");

test("Clipboard copies, distributes paste, cuts, and restores isolated history", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("one two\nthree four");
  const copiedSelections = TextSelectionSet.withPrimary([
    selection(0, 0, 0, 3),
    selection(1, 0, 1, 5),
  ], 1);
  using selections = new EditorSelectionController(model, copiedSelections);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 80, height: 40 });
  using input = new AlphaTextInputController(viewport, selections, {
    clipboard: { lineEnding: AlphaClipboardLineEnding.LF },
  });

  const copiedData = new MemoryClipboardData();
  const copy = clipboardEvent(dom.window, "copy", copiedData);
  input.element.dispatchEvent(copy);
  assert.equal(copy.defaultPrevented, true);
  assert.equal(copiedData.getData("text/plain"), "one\nthree");
  assert.deepEqual(
    JSON.parse(copiedData.getData(ALPHA_EDITOR_CLIPBOARD_MIME)),
    {
      version: 2,
      selectionTexts: ["one", "three"],
      pasteModes: [
        EditorClipboardPasteMode.Selection,
        EditorClipboardPasteMode.Selection,
      ],
    },
  );

  const pasteTargets = TextSelectionSet.withPrimary([
    selection(0, 4, 0, 7),
    selection(1, 6, 1, 10),
  ], 1);
  selections.setSelections(pasteTargets);
  const paste = clipboardEvent(dom.window, "paste", copiedData);
  input.element.dispatchEvent(paste);
  assert.equal(paste.defaultPrevented, true);
  assert.deepEqual({
    text: model.getText(),
    selections: selections.selections,
  }, {
    text: "one one\nthree three",
    selections: TextSelectionSet.withPrimary([
      caret(0, 7),
      caret(1, 11),
    ], 1),
  });

  selections.undo();
  assert.deepEqual({
    text: model.getText(),
    selections: selections.selections,
  }, {
    text: "one two\nthree four",
    selections: pasteTargets,
  });

  const cutData = new MemoryClipboardData();
  const cut = clipboardEvent(dom.window, "cut", cutData);
  input.element.dispatchEvent(cut);
  assert.equal(cut.defaultPrevented, true);
  assert.equal(cutData.getData("text/plain"), "two\nfour");
  assert.equal(model.getText(), "one \nthree ");
  selections.undo();
  assert.equal(model.getText(), "one two\nthree four");

  dom.window.close();
});

test("Clipboard repeats external text and copies an empty selection as a line", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("a b");
  using selections = new EditorSelectionController(
    model,
    TextSelectionSet.withPrimary([caret(0, 0), caret(0, 2)], 0),
  );
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 80, height: 20 });
  const input = new AlphaTextInputController(viewport, selections, {
    clipboard: { lineEnding: AlphaClipboardLineEnding.LF },
  });

  const externalData = new MemoryClipboardData();
  externalData.setData("text/plain", "X\r\nY");
  externalData.setData(ALPHA_EDITOR_CLIPBOARD_MIME, JSON.stringify({
    version: 1,
    selectionTexts: ["wrong count"],
  }));
  const paste = clipboardEvent(dom.window, "paste", externalData);
  input.element.dispatchEvent(paste);
  assert.equal(paste.defaultPrevented, true);
  assert.deepEqual({
    text: model.getText(),
    selections: selections.selections,
  }, {
    text: "X\nYa X\nYb",
    selections: TextSelectionSet.withPrimary([
      caret(1, 1),
      caret(2, 1),
    ], 0),
  });

  selections.setSelections(TextSelectionSet.single(caret(0, 0)));
  const emptyData = new MemoryClipboardData();
  const emptyCopy = clipboardEvent(dom.window, "copy", emptyData);
  input.element.dispatchEvent(emptyCopy);
  assert.equal(emptyCopy.defaultPrevented, true);
  assert.equal(emptyData.getData("text/plain"), "X\n");

  input.dispose();
  const disposedData = new MemoryClipboardData();
  disposedData.setData("text/plain", "ignored");
  const disposedPaste = clipboardEvent(dom.window, "paste", disposedData);
  input.element.dispatchEvent(disposedPaste);
  assert.equal(disposedPaste.defaultPrevented, false);
  assert.equal(model.getText(), "X\nYa X\nYb");

  dom.window.close();
});

test("Clipboard round-trips complete lines and preserves target columns", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("one\ntwo\nthree");
  using selections = new EditorSelectionController(
    model,
    TextSelectionSet.withPrimary([caret(0, 1), caret(2, 2)], 1),
  );
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 80, height: 40 });
  using input = new AlphaTextInputController(viewport, selections, {
    clipboard: { lineEnding: AlphaClipboardLineEnding.LF },
  });

  const lineData = new MemoryClipboardData();
  input.element.dispatchEvent(clipboardEvent(dom.window, "copy", lineData));
  assert.equal(lineData.getData("text/plain"), "one\nthree\n");
  assert.deepEqual(
    JSON.parse(lineData.getData(ALPHA_EDITOR_CLIPBOARD_MIME)),
    {
      version: 2,
      selectionTexts: ["one\n", "three\n"],
      pasteModes: [
        EditorClipboardPasteMode.Line,
        EditorClipboardPasteMode.Line,
      ],
    },
  );

  const targets = TextSelectionSet.withPrimary([
    caret(0, 2),
    caret(1, 1),
  ], 1);
  selections.setSelections(targets);
  input.element.dispatchEvent(clipboardEvent(dom.window, "paste", lineData));
  assert.deepEqual({
    text: model.getText(),
    selections: selections.selections,
  }, {
    text: "one\none\nthree\ntwo\nthree",
    selections: TextSelectionSet.withPrimary([
      caret(1, 2),
      caret(3, 1),
    ], 1),
  });

  selections.undo();
  assert.deepEqual({
    text: model.getText(),
    selections: selections.selections,
  }, {
    text: "one\ntwo\nthree",
    selections: targets,
  });

  selections.setSelections(TextSelectionSet.single(caret(1, 2)));
  const cutData = new MemoryClipboardData();
  input.element.dispatchEvent(clipboardEvent(dom.window, "cut", cutData));
  assert.deepEqual({
    clipboard: cutData.getData("text/plain"),
    text: model.getText(),
    selection: selections.selections.primary,
  }, {
    clipboard: "two\n",
    text: "one\nthree",
    selection: caret(1, 0),
  });
  selections.undo();
  assert.equal(model.getText(), "one\ntwo\nthree");

  dom.window.close();
});

test("Mixed line and selection metadata falls back to selection paste", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("a\nb");
  using selections = new EditorSelectionController(
    model,
    TextSelectionSet.withPrimary([
      caret(0, 1),
      selection(1, 0, 1, 1),
    ], 1),
  );
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  using input = new AlphaTextInputController(viewport, selections, {
    clipboard: { lineEnding: AlphaClipboardLineEnding.LF },
  });

  const data = new MemoryClipboardData();
  input.element.dispatchEvent(clipboardEvent(dom.window, "copy", data));
  assert.deepEqual(
    JSON.parse(data.getData(ALPHA_EDITOR_CLIPBOARD_MIME)).pasteModes,
    [EditorClipboardPasteMode.Line, EditorClipboardPasteMode.Selection],
  );

  selections.setSelections(TextSelectionSet.withPrimary([
    caret(0, 0),
    caret(1, 0),
  ], 1));
  input.element.dispatchEvent(clipboardEvent(dom.window, "paste", data));
  assert.deepEqual({
    text: model.getText(),
    selections: selections.selections,
  }, {
    text: "a\na\nbb",
    selections: TextSelectionSet.withPrimary([
      caret(1, 0),
      caret(2, 1),
    ], 1),
  });

  dom.window.close();
});

test("Empty-selection clipboard policy may explicitly preserve browser behavior", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("abc");
  using selections = new EditorSelectionController(
    model,
    TextSelectionSet.single(caret(0, 1)),
  );
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  using input = new AlphaTextInputController(viewport, selections, {
    clipboard: {
      lineEnding: AlphaClipboardLineEnding.LF,
      emptySelectionPolicy: EditorEmptySelectionClipboardPolicy.Ignore,
    },
  });

  const data = new MemoryClipboardData();
  const copy = clipboardEvent(dom.window, "copy", data);
  input.element.dispatchEvent(copy);
  assert.equal(copy.defaultPrevented, false);
  assert.equal(data.getData("text/plain"), "");

  dom.window.close();
});

function clipboardEvent(targetWindow: typeof browserEnvironment.window, type: "copy" | "cut" | "paste", clipboardData: MemoryClipboardData): ClipboardEvent {
  const event = new targetWindow.Event(type, {
    bubbles: true,
    cancelable: true,
  }) as unknown as ClipboardEvent;
  Object.defineProperty(event, "clipboardData", {
    configurable: true,
    value: clipboardData as unknown as DataTransfer,
  });
  return event;
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
