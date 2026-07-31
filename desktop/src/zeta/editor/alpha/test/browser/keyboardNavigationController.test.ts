import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { OperatingSystem } from "../../../../base/common/platform.js";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { resolveAlphaKeyboardNavigation } from "../../browser/keyboardNavigationController.js";
import { EditorCursorNavigationCommand, EditorCursorNavigationMode } from "../../common/cursorNavigation.js";
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

test("Keyboard navigation resolves Windows/Linux and macOS chords", () => {
  assert.deepEqual(resolveAlphaKeyboardNavigation(
    key("ArrowLeft"),
    OperatingSystem.Windows,
  ), {
    command: EditorCursorNavigationCommand.CharacterLeft,
    mode: EditorCursorNavigationMode.Move,
  });
  assert.deepEqual(resolveAlphaKeyboardNavigation(
    key("End", { shiftKey: true }),
    OperatingSystem.Linux,
  ), {
    command: EditorCursorNavigationCommand.LineEnd,
    mode: EditorCursorNavigationMode.Extend,
  });
  assert.deepEqual(resolveAlphaKeyboardNavigation(
    key("ArrowRight", { ctrlKey: true }),
    OperatingSystem.Windows,
  ), {
    command: EditorCursorNavigationCommand.WordRight,
    mode: EditorCursorNavigationMode.Move,
  });
  assert.deepEqual(resolveAlphaKeyboardNavigation(
    key("Home", { ctrlKey: true, shiftKey: true }),
    OperatingSystem.Windows,
  ), {
    command: EditorCursorNavigationCommand.DocumentStart,
    mode: EditorCursorNavigationMode.Extend,
  });
  assert.deepEqual(resolveAlphaKeyboardNavigation(
    key("ArrowLeft", { altKey: true, shiftKey: true }),
    OperatingSystem.Macintosh,
  ), {
    command: EditorCursorNavigationCommand.WordLeft,
    mode: EditorCursorNavigationMode.Extend,
  });
  assert.deepEqual(resolveAlphaKeyboardNavigation(
    key("ArrowRight", { metaKey: true }),
    OperatingSystem.Macintosh,
  ), {
    command: EditorCursorNavigationCommand.LineEnd,
    mode: EditorCursorNavigationMode.Move,
  });
  assert.deepEqual(resolveAlphaKeyboardNavigation(
    key("ArrowDown", { metaKey: true }),
    OperatingSystem.Macintosh,
  ), {
    command: EditorCursorNavigationCommand.DocumentEnd,
    mode: EditorCursorNavigationMode.Move,
  });
  assert.equal(resolveAlphaKeyboardNavigation(
    key("ArrowLeft", { altKey: true }),
    OperatingSystem.Windows,
  ), undefined);
  assert.equal(resolveAlphaKeyboardNavigation(
    key("ArrowLeft", { isComposing: true }),
    OperatingSystem.Windows,
  ), undefined);
  assert.equal(resolveAlphaKeyboardNavigation(
    key("ArrowLeft", { altGraphKey: true }),
    OperatingSystem.Windows,
  ), undefined);
});

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
  KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { AlphaEditorViewport } = await import("../../browser/alphaEditorViewport.js");
const { AlphaKeyboardNavigationController } = await import("../../browser/keyboardNavigationController.js");

test("Keyboard controller retains columns, routes multi-selection, and reveals primary", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel([
    "abcdef",
    "x",
    "abcdef",
    "line",
    "line",
    "line",
    "line",
    "line",
    "line",
    "abcdefghijklmnopqrstuvwxyz",
  ].join("\n"));
  using selections = new EditorSelectionController(
    model,
    TextSelectionSet.single(caret(0, 5)),
  );
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  viewport.layout({ width: 80, height: 40 });
  using keyboard = new AlphaKeyboardNavigationController(
    viewport,
    selections,
    { operatingSystem: OperatingSystem.Windows },
  );

  const firstDown = keyboardEvent(dom.window, "ArrowDown");
  viewport.element.dispatchEvent(firstDown);
  viewport.element.dispatchEvent(keyboardEvent(dom.window, "ArrowDown"));
  assert.equal(firstDown.defaultPrevented, true);
  assert.deepEqual(selections.selections.primary, caret(2, 5));

  selections.setSelections(TextSelectionSet.single(caret(0, 2)));
  viewport.element.dispatchEvent(keyboardEvent(dom.window, "ArrowDown"));
  viewport.element.dispatchEvent(keyboardEvent(dom.window, "ArrowDown"));
  assert.deepEqual(selections.selections.primary, caret(2, 2));

  viewport.element.dispatchEvent(keyboardEvent(
    dom.window,
    "ArrowRight",
    { shiftKey: true },
  ));
  assert.deepEqual(
    selections.selections.primary,
    TextSelection.from(TextPosition.at(2, 2), TextPosition.at(2, 3)),
  );

  const textKey = keyboardEvent(dom.window, "a");
  viewport.element.dispatchEvent(textKey);
  assert.equal(textKey.defaultPrevented, false);
  assert.deepEqual(
    selections.selections.primary,
    TextSelection.from(TextPosition.at(2, 2), TextPosition.at(2, 3)),
  );

  viewport.element.dispatchEvent(keyboardEvent(
    dom.window,
    "End",
    { ctrlKey: true },
  ));
  assert.deepEqual(selections.selections.primary, caret(9, 26));
  assert.equal(viewport.viewportLayout.scrollPosition.top, 160);
  assert.ok(viewport.viewportLayout.scrollPosition.left > 0);

  viewport.element.dispatchEvent(keyboardEvent(
    dom.window,
    "Home",
    { ctrlKey: true },
  ));
  assert.deepEqual(selections.selections.primary, caret(0, 0));
  assert.deepEqual(viewport.viewportLayout.scrollPosition, {
    left: 0,
    top: 0,
  });

  viewport.element.dispatchEvent(keyboardEvent(dom.window, "PageDown"));
  assert.deepEqual(selections.selections.primary, caret(2, 0));
  assert.equal(viewport.viewportLayout.scrollPosition.top, 20);
  viewport.element.dispatchEvent(keyboardEvent(dom.window, "PageUp"));
  assert.deepEqual(selections.selections.primary, caret(0, 0));
  assert.equal(viewport.viewportLayout.scrollPosition.top, 0);

  selections.setSelections(TextSelectionSet.withPrimary([
    caret(0, 1),
    caret(1, 1),
  ], 1));
  viewport.element.dispatchEvent(keyboardEvent(dom.window, "ArrowDown"));
  assert.deepEqual(selections.selections, TextSelectionSet.withPrimary([
    caret(1, 1),
    caret(2, 1),
  ], 1));

  keyboard.dispose();
  const disposedSelections = selections.selections;
  viewport.element.dispatchEvent(keyboardEvent(dom.window, "ArrowDown"));
  assert.equal(selections.selections, disposedSelections);

  dom.window.close();
});

test("Keyboard controller rejects cross-model wiring and invalid OS options", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("alpha");
  using otherModel = new TextModel("beta");
  using selections = new EditorSelectionController(
    otherModel,
    TextSelectionSet.single(caret(0, 0)),
  );
  using ownSelections = new EditorSelectionController(
    model,
    TextSelectionSet.single(caret(0, 0)),
  );
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
  });

  assert.throws(
    () => new AlphaKeyboardNavigationController(viewport, selections),
    /must share one text model/,
  );
  assert.throws(
    () => new AlphaKeyboardNavigationController(
      viewport,
      ownSelections,
      { operatingSystem: "plan9" as OperatingSystem },
    ),
    /Unknown Alpha keyboard operating system/,
  );

  dom.window.close();
});

interface KeyOptions {
  readonly ctrlKey?: boolean;
  readonly shiftKey?: boolean;
  readonly altKey?: boolean;
  readonly metaKey?: boolean;
  readonly altGraphKey?: boolean;
  readonly isComposing?: boolean;
}

function key(keyValue: string, options: KeyOptions = {}) {
  return {
    key: keyValue,
    ctrlKey: options.ctrlKey ?? false,
    shiftKey: options.shiftKey ?? false,
    altKey: options.altKey ?? false,
    metaKey: options.metaKey ?? false,
    altGraphKey: options.altGraphKey ?? false,
    isComposing: options.isComposing ?? false,
  };
}

function keyboardEvent(
  targetWindow: typeof browserEnvironment.window,
  keyValue: string,
  options: KeyOptions = {},
): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key: keyValue,
    ctrlKey: options.ctrlKey,
    shiftKey: options.shiftKey,
    altKey: options.altKey,
    metaKey: options.metaKey,
    isComposing: options.isComposing,
  }) as unknown as KeyboardEvent;
}

function caret(lineIndex: number, columnIndex: number): TextSelection {
  return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}
