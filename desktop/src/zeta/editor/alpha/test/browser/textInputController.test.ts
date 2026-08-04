import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { OperatingSystem } from "../../../../base/common/platform.js";
import { type AlphaTextMeasurer } from "../../browser/view/fontMetrics.js";
import { EditorIndentationKind } from "../../contrib/indentation/common/indentation.js";
import { EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { registerBuiltinLanguageConfigurations } from "../../common/languages/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry, LanguageIndentAction } from "../../common/languages/languageConfiguration.js";
import { LanguageLexicalContextIndex } from "../../common/languages/languageLexicalContext.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

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
  InputEvent: browserEnvironment.window.InputEvent,
  KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { AlphaEditorTextDirection, AlphaEditorViewport } = await import("../../browser/view/editorViewport.js");
const { AlphaKeyboardNavigationController } = await import("../../browser/input/keyboardNavigationController.js");
const { AlphaTextInputController } = await import("../../browser/input/textInputController.js");

test("Textarea routes navigation, typing, history, deletion, and Tab", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("ab\ncd");
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
  viewport.layout({ width: 100, height: 40 });
  using keyboard = new AlphaKeyboardNavigationController(
    viewport,
    selections,
    { operatingSystem: OperatingSystem.Windows },
  );
  const input = new AlphaTextInputController(viewport, selections);

  viewport.element.focus();
  assert.equal(dom.window.document.activeElement, input.element);
  assert.equal(viewport.element.classList.contains("input-focused"), true);

  input.element.dispatchEvent(keyboardEvent(dom.window, "ArrowRight"));
  assert.deepEqual(selections.selections.primary, caret(0, 2));

  const firstText = beforeInput(dom.window, "insertText", "X");
  input.element.dispatchEvent(firstText);
  input.element.dispatchEvent(beforeInput(dom.window, "insertText", "Y"));
  assert.equal(firstText.defaultPrevented, true);
  assert.deepEqual({
    text: model.getText(),
    selection: selections.selections.primary,
  }, {
    text: "abXY\ncd",
    selection: caret(0, 4),
  });

  input.element.dispatchEvent(beforeInput(dom.window, "historyUndo"));
  assert.deepEqual({
    text: model.getText(),
    selection: selections.selections.primary,
  }, {
    text: "ab\ncd",
    selection: caret(0, 2),
  });
  input.element.dispatchEvent(beforeInput(dom.window, "historyRedo"));
  assert.equal(model.getText(), "abXY\ncd");

  input.element.dispatchEvent(beforeInput(dom.window, "insertLineBreak"));
  const tab = keyboardEvent(dom.window, "Tab");
  input.element.dispatchEvent(tab);
  assert.equal(tab.defaultPrevented, true);
  assert.deepEqual({
    text: model.getText(),
    selection: selections.selections.primary,
  }, {
    text: "abXY\n\t\ncd",
    selection: caret(1, 1),
  });

  input.element.dispatchEvent(beforeInput(
    dom.window,
    "deleteContentBackward",
  ));
  assert.deepEqual({
    text: model.getText(),
    selection: selections.selections.primary,
  }, {
    text: "abXY\n\ncd",
    selection: caret(1, 0),
  });

  selections.setSelections(TextSelectionSet.withPrimary([
    caret(0, 0),
    caret(2, 2),
  ], 1));
  input.element.dispatchEvent(beforeInput(dom.window, "insertText", "!"));
  assert.deepEqual({
    text: model.getText(),
    selections: selections.selections,
  }, {
    text: "!abXY\n\ncd!",
    selections: TextSelectionSet.withPrimary([
      caret(0, 1),
      caret(2, 3),
    ], 1),
  });

  selections.setSelections(TextSelectionSet.single(caret(0, 0)));
  input.element.dispatchEvent(beforeInput(
    dom.window,
    "deleteContentForward",
  ));
  assert.equal(model.getText(), "abXY\n\ncd!");

  const composing = beforeInput(
    dom.window,
    "insertCompositionText",
    "中",
    true,
  );
  input.element.dispatchEvent(composing);
  assert.equal(composing.defaultPrevented, false);
  assert.equal(model.getText(), "abXY\n\ncd!");
  const paste = beforeInput(dom.window, "insertFromPaste", "paste");
  input.element.dispatchEvent(paste);
  assert.equal(paste.defaultPrevented, false);

  input.element.value = "transient";
  input.element.dispatchEvent(new dom.window.InputEvent("input", {
    bubbles: true,
    inputType: "insertText",
    data: "transient",
  }));
  assert.equal(input.element.value, "");

  input.element.blur();
  assert.equal(viewport.element.classList.contains("input-focused"), false);
  input.dispose();
  assert.equal(viewport.element.contains(input.element), false);
  const disposedText = model.getText();
  input.element.dispatchEvent(beforeInput(dom.window, "insertText", "Z"));
  assert.equal(model.getText(), disposedText);

  dom.window.close();
});

test("Textarea routes browser soft-line deletion through Alpha commands", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("alpha\nbeta");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 3)));
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
  using input = new AlphaTextInputController(viewport, selections);

  const backward = beforeInput(dom.window, "deleteSoftLineBackward");
  input.element.dispatchEvent(backward);
  assert.equal(backward.defaultPrevented, true);
  assert.equal(model.getText(), "ha\nbeta");
  selections.setSelections(TextSelectionSet.single(caret(1, 1)));
  input.element.dispatchEvent(beforeInput(dom.window, "deleteSoftLineForward"));
  assert.equal(model.getText(), "ha\nb");

  dom.window.close();
});

test("Textarea accepts an isolated composing dead-key commit without a composition session", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("e");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 1)));
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
  using input = new AlphaTextInputController(viewport, selections);

  const commit = beforeInput(dom.window, "insertText", "́", true);
  input.element.dispatchEvent(commit);
  assert.equal(commit.defaultPrevented, true);
  assert.equal(model.getText(), "é");
  input.element.value = "é";
  input.element.dispatchEvent(new dom.window.InputEvent("input", {
    bubbles: true,
    inputType: "insertText",
    data: "́",
    isComposing: true,
  }));
  assert.equal(input.element.value, "");
  dom.window.close();
});

test("Textarea mirrors the focused document and primary selection for assistive technology", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("alpha\nbeta");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 2)));
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
  using input = new AlphaTextInputController(viewport, selections, { ariaLabel: "Source file" });

  input.focus();
  assert.equal(input.element.getAttribute("aria-roledescription"), "code editor");
  assert.equal(input.element.getAttribute("aria-multiline"), "true");
  assert.equal(input.element.value, "alpha\nbeta");
  assert.equal(input.element.selectionStart, 2);
  assert.equal(input.element.selectionEnd, 2);

  selections.setSelections(TextSelectionSet.single(TextSelection.from(TextPosition.at(1, 3), TextPosition.at(1, 1))));
  await Promise.resolve();
  assert.equal(input.element.selectionStart, 7);
  assert.equal(input.element.selectionEnd, 9);
  assert.equal(input.element.selectionDirection, "backward");

  selections.setSelections(TextSelectionSet.withPrimary([
    caret(0, 0),
    caret(1, 2),
  ], 1));
  await Promise.resolve();
  assert.equal(input.element.getAttribute("aria-description"), "2 selections. Primary at line 2, column 3.");

  input.element.setSelectionRange(1, 4, "forward");
  input.element.dispatchEvent(new dom.window.Event("select"));
  assert.deepEqual(selections.selections, TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 1), TextPosition.at(0, 4))));

  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 5)),
    text: "!",
  }]);
  await Promise.resolve();
  assert.equal(input.element.value, "alpha!\nbeta");

  input.element.blur();
  assert.equal(input.element.value, "");
  dom.window.close();
});

test("Textarea inherits the viewport direction for macOS accessibility text services", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("שלום");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 0)));
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
    textDirection: AlphaEditorTextDirection.RightToLeft,
  });
  using input = new AlphaTextInputController(viewport, selections);

  assert.equal(input.element.dir, "rtl");
  dom.window.close();
});

test("Textarea toggles transient overtype mode for ordinary input", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("a😊bc");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 1)));
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
  using input = new AlphaTextInputController(viewport, selections);

  const enable = keyboardEvent(dom.window, "Insert");
  input.element.dispatchEvent(enable);
  assert.equal(enable.defaultPrevented, true);
  assert.equal(input.overtyping, true);
  assert.equal(viewport.element.classList.contains("overtype"), true);
  input.element.dispatchEvent(beforeInput(dom.window, "insertText", "X"));
  assert.equal(model.getText(), "aXbc");
  assert.deepEqual(selections.selections.primary, caret(0, 2));

  input.element.dispatchEvent(keyboardEvent(dom.window, "Insert"));
  assert.equal(input.overtyping, false);
  input.element.dispatchEvent(beforeInput(dom.window, "insertText", "Y"));
  assert.equal(model.getText(), "aXYbc");
  dom.window.close();
});

test("Textarea rejects cross-model wiring without owning either model", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("alpha");
  using otherModel = new TextModel("beta");
  using selections = new EditorSelectionController(
    otherModel,
    TextSelectionSet.single(caret(0, 0)),
  );
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
  });

  assert.throws(
    () => new AlphaTextInputController(viewport, selections),
    /must share one text model/,
  );
  using compatibleSelections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 0)));
  assert.throws(() => new AlphaTextInputController(viewport, compatibleSelections, {
    language: {
      languageId: "*",
      configurations: { getLanguageConfiguration: () => { throw new Error("unreachable"); } },
    },
  }), /Language ID/);
  assert.throws(() => new AlphaTextInputController(viewport, compatibleSelections, {
    language: {
      languageId: "typescript",
      configurations: {} as LanguageConfigurationRegistry,
    },
  }), /configuration source/);
  using lexicalModel = new TextModel("");
  using lexicalConfigurations = new LanguageConfigurationRegistry();
  using lexicalContext = new LanguageLexicalContextIndex(lexicalModel, "typescript", lexicalConfigurations);
  assert.throws(() => new AlphaTextInputController(viewport, compatibleSelections, {
    language: {
      languageId: "typescript",
      configurations: { getLanguageConfiguration: () => { throw new Error("unreachable"); } },
      lexicalContext,
    },
  }), /lexical context/);
  assert.throws(() => new AlphaTextInputController(viewport, compatibleSelections, {
    indentation: { tabSize: 0 },
  }), /tab size/);
  model.applyEdits([{
    range: TextRange.emptyAt(model.positionAt(5)),
    text: " editor",
  }]);
  assert.equal(model.getText(), "alpha editor");

  dom.window.close();
});

test("Textarea applies current language pair configuration through editor commands", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("item");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 4)));
  using configurations = new LanguageConfigurationRegistry();
  using builtins = registerBuiltinLanguageConfigurations(configurations);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  using input = new AlphaTextInputController(viewport, selections, {
    language: {
      languageId: "typescript",
      configurations,
    },
  });

  input.element.dispatchEvent(beforeInput(dom.window, "insertText", "("));
  assert.equal(model.getText(), "item()");
  assert.deepEqual(selections.selections.primary, caret(0, 5));
  const pairVersion = model.version;
  input.element.dispatchEvent(beforeInput(dom.window, "insertText", ")"));
  assert.equal(model.getText(), "item()");
  assert.equal(model.version, pairVersion);
  assert.deepEqual(selections.selections.primary, caret(0, 6));

  input.element.dispatchEvent(beforeInput(dom.window, "insertText", "["));
  assert.equal(model.getText(), "item()[]");
  input.element.dispatchEvent(beforeInput(dom.window, "deleteContentBackward"));
  assert.equal(model.getText(), "item()");
  assert.deepEqual(selections.selections.primary, caret(0, 6));

  selections.setSelections(TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 4))));
  input.element.dispatchEvent(beforeInput(dom.window, "insertText", "\""));
  assert.equal(model.getText(), "\"item\"()");
  assert.deepEqual(selections.selections.primary, TextSelection.from(TextPosition.at(0, 1), TextPosition.at(0, 5)));

  using override = configurations.register("typescript", {
    autoClosingPairs: null,
    surroundingPairs: null,
  }, { priority: 10 });
  selections.setSelections(TextSelectionSet.single(caret(0, model.getText().length)));
  input.element.dispatchEvent(beforeInput(dom.window, "insertText", "{"));
  assert.equal(model.getText(), "\"item\"(){");
  assert.deepEqual(selections.selections.primary, caret(0, model.getText().length));

  dom.window.close();
});

test("Textarea does not trust matching pairs that it did not auto-close", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("()");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 1)));
  using configurations = new LanguageConfigurationRegistry();
  using builtins = registerBuiltinLanguageConfigurations(configurations);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  using input = new AlphaTextInputController(viewport, selections, {
    language: {
      languageId: "typescript",
      configurations,
    },
  });

  input.element.dispatchEvent(beforeInput(dom.window, "insertText", ")"));
  assert.equal(model.getText(), "())");
  assert.deepEqual(selections.selections.primary, caret(0, 2));

  selections.setSelections(TextSelectionSet.single(caret(0, 1)));
  input.element.dispatchEvent(beforeInput(dom.window, "deleteContentBackward"));
  assert.equal(model.getText(), "))");
  assert.deepEqual(selections.selections.primary, caret(0, 0));

  dom.window.close();
});

test("Textarea applies current on-enter rules with editor-owned indentation", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("{}");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 1)));
  using configurations = new LanguageConfigurationRegistry();
  using builtins = registerBuiltinLanguageConfigurations(configurations);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  using input = new AlphaTextInputController(viewport, selections, {
    indentation: {
      kind: EditorIndentationKind.Spaces,
      tabSize: 2,
    },
    language: {
      languageId: "typescript",
      configurations,
    },
  });

  input.element.dispatchEvent(beforeInput(dom.window, "insertLineBreak"));
  assert.equal(model.getText(), "{\n  \n}");
  assert.deepEqual(selections.selections.primary, caret(1, 2));

  input.element.dispatchEvent(beforeInput(dom.window, "historyUndo"));
  assert.equal(model.getText(), "{}");
  assert.deepEqual(selections.selections.primary, caret(0, 1));

  using override = configurations.register("typescript", {
    onEnterRules: [{
      beforeText: /\{$/,
      afterText: /^\}/,
      action: {
        indentAction: LanguageIndentAction.None,
        appendText: "custom",
      },
    }],
  }, { priority: 10 });
  input.element.dispatchEvent(beforeInput(dom.window, "insertParagraph"));
  assert.equal(model.getText(), "{\ncustom}");
  assert.deepEqual(selections.selections.primary, caret(1, 6));

  dom.window.close();
});

test("Textarea Enter ignores structural brackets inside lexical string tokens", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("const value = \"{\"");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, model.getText().length)));
  using configurations = new LanguageConfigurationRegistry();
  using builtins = registerBuiltinLanguageConfigurations(configurations);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  using input = new AlphaTextInputController(viewport, selections, {
    indentation: {
      kind: EditorIndentationKind.Spaces,
      tabSize: 2,
    },
    language: {
      languageId: "typescript",
      configurations,
    },
  });

  input.element.dispatchEvent(beforeInput(dom.window, "insertLineBreak"));
  assert.equal(model.getText(), "const value = \"{\"\n");
  assert.deepEqual(selections.selections.primary, caret(1, 0));

  dom.window.close();
});

test("Textarea respects auto-closing notIn inside lexical string tokens", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector("main");
  assert.ok(container);
  using model = new TextModel("\"value \"");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(caret(0, 7)));
  using configurations = new LanguageConfigurationRegistry();
  using builtins = registerBuiltinLanguageConfigurations(configurations);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    selectionController: selections,
  });
  using input = new AlphaTextInputController(viewport, selections, {
    language: {
      languageId: "typescript",
      configurations,
    },
  });

  input.element.dispatchEvent(beforeInput(dom.window, "insertText", "'"));
  assert.equal(model.getText(), "\"value '\"");
  assert.deepEqual(selections.selections.primary, caret(0, 8));

  dom.window.close();
});

function beforeInput(
  targetWindow: typeof browserEnvironment.window,
  inputType: string,
  data: string | null = null,
  isComposing = false,
): InputEvent {
  return new targetWindow.InputEvent("beforeinput", {
    bubbles: true,
    cancelable: true,
    inputType,
    data,
    isComposing,
  }) as unknown as InputEvent;
}

function keyboardEvent(
  targetWindow: typeof browserEnvironment.window,
  key: string,
): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key,
  }) as unknown as KeyboardEvent;
}

function caret(lineIndex: number, columnIndex: number): TextSelection {
  return TextSelection.collapsedAt(TextPosition.at(lineIndex, columnIndex));
}
