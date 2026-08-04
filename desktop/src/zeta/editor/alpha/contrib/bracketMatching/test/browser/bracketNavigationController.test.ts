import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type AlphaTextMeasurer } from "../../../../browser/view/fontMetrics.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { LanguageConfigurationRegistry } from "../../../../common/languages/languageConfiguration.js";
import { LanguageBracketMatcher } from "../../common/bracketMatching.js";
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
  KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { AlphaEditorViewport } = await import("../../../../browser/view/editorViewport.js");
const { AlphaBracketNavigationController } = await import("../../browser/bracketNavigationController.js");

test("Go-to-bracket shortcut uses the Alpha lexical bracket matcher", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("(value)");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using configurations = configurationsForBrackets();
  using matcher = new LanguageBracketMatcher(model, "typescript", configurations);
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
  viewport.layout({ width: 200, height: 60 });
  const input = dom.window.document.createElement("textarea");
  container.append(input);
  using controller = new AlphaBracketNavigationController(input, viewport, selections, matcher);

  const jump = keydown(dom.window, "\\", { ctrlKey: true, shiftKey: true });
  input.dispatchEvent(jump);
  assert.equal(jump.defaultPrevented, true);
  assert.deepEqual(selections.selections.primary, TextSelection.collapsedAt(TextPosition.at(0, 6)));

  dom.window.close();
});

test("Bracket navigation controller rejects cross-model wiring and unrelated chords", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("()");
  using other = new TextModel("()");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using configurations = configurationsForBrackets();
  using matcher = new LanguageBracketMatcher(model, "typescript", configurations);
  using otherMatcher = new LanguageBracketMatcher(other, "typescript", configurations);
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer() });
  const input = dom.window.document.createElement("textarea");
  container.append(input);
  using controller = new AlphaBracketNavigationController(input, viewport, selections, matcher);
  const unrelated = keydown(dom.window, "\\", { ctrlKey: true });
  input.dispatchEvent(unrelated);
  assert.equal(unrelated.defaultPrevented, false);
  assert.throws(() => new AlphaBracketNavigationController(input, viewport, selections, otherMatcher), /must share one text model/);

  dom.window.close();
});

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

function configurationsForBrackets(): LanguageConfigurationRegistry {
  const configurations = new LanguageConfigurationRegistry();
  configurations.register("typescript", { brackets: [{ open: "(", close: ")" }] });
  return configurations;
}

function keydown(targetWindow: typeof browserEnvironment.window, key: string, options: KeyboardEventInit = {}): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", { bubbles: true, cancelable: true, key, ...options }) as unknown as KeyboardEvent;
}
