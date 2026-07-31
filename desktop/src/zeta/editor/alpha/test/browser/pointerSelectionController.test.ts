import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { EditorSelectionController } from "../../common/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition, TextRange } from "../../common/text.js";
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

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { AlphaEditorViewport } = await import(
  "../../browser/alphaEditorViewport.js"
);
const { AlphaPointerSelectionController } = await import(
  "../../browser/pointerSelectionController.js"
);

test("Pointer selection supports clicks, Shift, drag, gutter, and cancellation", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("abcd\nefgh\nijkl\nmnop");
  using selections = new EditorSelectionController(
    model,
    TextSelectionSet.single(
      TextSelection.collapsedAt(TextPosition.at(0, 0)),
    ),
  );
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 200, height: 80 });
  viewport.element.getBoundingClientRect = () => editorBounds();
  const captured = new Set<number>();
  viewport.element.setPointerCapture = pointerId => captured.add(pointerId);
  viewport.element.hasPointerCapture = pointerId => captured.has(pointerId);
  viewport.element.releasePointerCapture = pointerId => {
    captured.delete(pointerId);
  };
  const pointer = new AlphaPointerSelectionController(viewport, selections);

  const click = pointerEvent(dom.window, "pointerdown", 148, 75, {
    pointerId: 1,
  });
  viewport.element.dispatchEvent(click);
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointerup",
    148,
    75,
    { pointerId: 1 },
  ));
  assert.equal(click.defaultPrevented, true);
  assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(
    TextPosition.at(1, 1),
  ));
  assert.deepEqual([...captured], []);

  viewport.element.dispatchEvent(pointerEvent(
    dom.window,
    "pointerdown",
    158,
    105,
    { pointerId: 2, shiftKey: true },
  ));
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointerup",
    158,
    105,
    { pointerId: 2, shiftKey: true },
  ));
  assert.deepEqual(selections.selections.primary, TextSelection.from(
    TextPosition.at(1, 1),
    TextPosition.at(2, 2),
  ));

  viewport.element.dispatchEvent(pointerEvent(
    dom.window,
    "pointerdown",
    148,
    55,
    { pointerId: 3 },
  ));
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointermove",
    168,
    105,
    { pointerId: 99 },
  ));
  assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(
    TextPosition.at(0, 1),
  ));
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointermove",
    168,
    105,
    { pointerId: 3 },
  ));
  assert.deepEqual(selections.selections.primary, TextSelection.from(
    TextPosition.at(0, 1),
    TextPosition.at(2, 3),
  ));
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointerup",
    168,
    105,
    { pointerId: 3 },
  ));

  viewport.element.dispatchEvent(pointerEvent(
    dom.window,
    "pointerdown",
    110,
    75,
    { pointerId: 4 },
  ));
  assert.deepEqual(selections.selections.primary, TextSelection.from(
    TextPosition.at(1, 0),
    TextPosition.at(2, 0),
  ));
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointermove",
    110,
    115,
    { pointerId: 4 },
  ));
  assert.deepEqual(selections.selections.primary, TextSelection.from(
    TextPosition.at(1, 0),
    TextPosition.at(3, 4),
  ));
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointerup",
    110,
    115,
    { pointerId: 4 },
  ));

  viewport.element.dispatchEvent(pointerEvent(
    dom.window,
    "pointerdown",
    110,
    95,
    { pointerId: 5 },
  ));
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointermove",
    110,
    55,
    { pointerId: 5 },
  ));
  assert.deepEqual(selections.selections.primary, TextSelection.from(
    TextPosition.at(3, 0),
    TextPosition.at(0, 0),
  ));
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointercancel",
    110,
    55,
    { pointerId: 5 },
  ));
  const cancelledSelection = selections.selections;
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointermove",
    168,
    115,
    { pointerId: 5 },
  ));
  assert.equal(selections.selections, cancelledSelection);
  assert.deepEqual([...captured], []);

  selections.setSelections(TextSelectionSet.single(TextSelection.from(
    TextPosition.at(1, 2),
    TextPosition.at(1, 2),
  )));
  viewport.element.dispatchEvent(pointerEvent(
    dom.window,
    "pointerdown",
    110,
    95,
    { pointerId: 6, shiftKey: true },
  ));
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointerup",
    110,
    95,
    { pointerId: 6, shiftKey: true },
  ));
  assert.deepEqual(selections.selections.primary, TextSelection.from(
    TextPosition.at(1, 2),
    TextPosition.at(3, 0),
  ));

  pointer.dispose();
  const disposedSelection = selections.selections;
  viewport.element.dispatchEvent(pointerEvent(
    dom.window,
    "pointerdown",
    148,
    55,
    { pointerId: 7 },
  ));
  assert.equal(selections.selections, disposedSelection);

  dom.window.close();
});

test("Pointer and viewport selection wiring rejects different text models", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("alpha");
  using otherModel = new TextModel("beta");
  using selections = new EditorSelectionController(
    otherModel,
    TextSelectionSet.single(
      TextSelection.collapsedAt(TextPosition.at(0, 0)),
    ),
  );
  assert.throws(() => new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  }), /must share one text model/);
  assert.equal(container.childElementCount, 0);

  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
  });
  assert.throws(
    () => new AlphaPointerSelectionController(viewport, selections),
    /must share one text model/,
  );
  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 5)),
    text: " editor",
  }]);
  assert.equal(model.getText(), "alpha editor");

  dom.window.close();
});

test("Pointer drag anchor tracks model edits and window blur ends capture", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("abc\ndef");
  using selections = new EditorSelectionController(
    model,
    TextSelectionSet.single(
      TextSelection.collapsedAt(TextPosition.at(0, 0)),
    ),
  );
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 200, height: 40 });
  viewport.element.getBoundingClientRect = () => ({
    ...editorBounds(),
    bottom: 90,
    height: 40,
  });
  const captured = new Set<number>();
  viewport.element.setPointerCapture = pointerId => captured.add(pointerId);
  viewport.element.hasPointerCapture = pointerId => captured.has(pointerId);
  viewport.element.releasePointerCapture = pointerId => {
    captured.delete(pointerId);
  };
  using pointer = new AlphaPointerSelectionController(viewport, selections);

  viewport.element.dispatchEvent(pointerEvent(
    dom.window,
    "pointerdown",
    148,
    55,
    { pointerId: 8 },
  ));
  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 0)),
    text: "X",
  }]);
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointermove",
    158,
    75,
    { pointerId: 8 },
  ));
  assert.deepEqual(selections.selections.primary, TextSelection.from(
    TextPosition.at(0, 2),
    TextPosition.at(1, 2),
  ));
  assert.deepEqual([...captured], [8]);

  dom.window.dispatchEvent(new dom.window.Event("blur"));
  assert.deepEqual([...captured], []);
  const blurredSelection = selections.selections;
  dom.window.dispatchEvent(pointerEvent(
    dom.window,
    "pointermove",
    168,
    75,
    { pointerId: 8 },
  ));
  assert.equal(selections.selections, blurredSelection);

  dom.window.close();
});

interface PointerEventOptions {
  readonly pointerId: number;
  readonly shiftKey?: boolean;
}

function pointerEvent(
  targetWindow: typeof browserEnvironment.window,
  type: string,
  clientX: number,
  clientY: number,
  options: PointerEventOptions,
): PointerEvent {
  const event = new targetWindow.MouseEvent(type, {
    bubbles: true,
    cancelable: true,
    button: 0,
    buttons: type === "pointerup" || type === "pointercancel" ? 0 : 1,
    clientX,
    clientY,
    shiftKey: options.shiftKey,
  });
  Object.defineProperty(event, "pointerId", {
    configurable: true,
    value: options.pointerId,
  });
  return event as unknown as PointerEvent;
}

function editorBounds(): DOMRect {
  return {
    x: 100,
    y: 50,
    left: 100,
    top: 50,
    right: 300,
    bottom: 130,
    width: 200,
    height: 80,
    toJSON: () => ({}),
  };
}
