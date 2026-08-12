import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type TextMeasurer } from "../../browser/view/fontMetrics.js";
import { EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { EditorFoldingModel } from "../../contrib/folding/browser/foldingModel.js";
import { EditorHiddenRangeModel } from "../../contrib/folding/browser/hiddenRangeModel.js";
import { TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

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

const { EditorViewport } = await import(
  "../../browser/view/editorViewport.js"
);
const { EditorMinimap } = await import(
  "../../browser/view/editorViewport.js"
);
const { EditorTextDirection } = await import(
  "../../browser/view/editorViewport.js"
);
const { EditorLineWrapping } = await import(
  "../../browser/view/visualLineProjection.js"
);

test("EditorViewport projects the initial virtual line window", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel([
    "<strong>not markup</strong>",
    ...lines(99),
  ].join("\n"));
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    overscanLineCount: 2,
    ariaLabel: "Read-only source",
    textMeasurer: fixedTextMeasurer(),
  });

  viewport.layout({ width: 300, height: 100 });
  const rows = lineElements(viewport.element);

  assert.equal(viewport.element.getAttribute("role"), "region");
  assert.equal(viewport.element.getAttribute("aria-label"), "Read-only source");
  assert.equal(viewport.element.tabIndex, 0);
  assert.equal(viewport.element.parentElement, container);
  assert.equal(viewport.element.querySelector("strong"), null);
  assert.equal(rows.length, 7);
  assert.equal(rows[0]?.dataset.lineIndex, "0");
  assert.equal(
    lineText(rows[0]).textContent,
    "<strong>not markup</strong>",
  );
  assert.equal(lineNumber(rows[0]).textContent, "1");
  assert.equal(rows[0]?.style.height, "20px");
  assert.equal(rows[6]?.dataset.lineIndex, "6");
  assert.equal(lineNumber(rows[6]).textContent, "7");
  assert.equal(
    viewport.element.style.getPropertyValue(
      "--aster-editor-gutter-width",
    ),
    "40px",
  );
  assert.equal(
    requiredElement(viewport.element, ".aster-editor-content").style.height,
    "2000px",
  );

  dom.window.close();
});

test("EditorViewport gives browser text shaping an explicit paragraph direction", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("שלום alpha");
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
    textDirection: EditorTextDirection.RightToLeft,
  });
  viewport.layout({ width: 300, height: 40 });

  assert.equal(viewport.editorTextDirection, EditorTextDirection.RightToLeft);
  assert.equal(viewport.element.dir, "rtl");
  assert.equal(viewport.element.classList.contains("aster-editor-direction-rtl"), true);
  assert.equal(lineText(requiredLine(viewport.element, 0)).dir, "rtl");
  dom.window.close();
});

test("EditorViewport uses browser range geometry for RTL selections and carets", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("abc אבג");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
    selectionController: selections,
    textDirection: EditorTextDirection.RightToLeft,
  });
  viewport.layout({ width: 300, height: 40 });
  const line = requiredLine(viewport.element, 0);
  Object.defineProperty(line, "getBoundingClientRect", {
    configurable: true,
    value: () => testRectangle(100, 0, 300),
  });
  const createRange = dom.window.document.createRange.bind(dom.window.document);
  Object.defineProperty(dom.window.document, "createRange", {
    configurable: true,
    value: () => {
      const range = createRange();
      Object.defineProperty(range, "getClientRects", {
        configurable: true,
        value: () => [testRectangle(150, 0, 20), testRectangle(120, 0, 15)],
      });
      Object.defineProperty(range, "getBoundingClientRect", {
        configurable: true,
        value: () => testRectangle(135, 0, 0),
      });
      return range;
    },
  });
  selections.setSelections(TextSelectionSet.single(TextSelection.from(TextPosition.at(0, 0), TextPosition.at(0, 3))));

  const selectionElements = [...line.querySelectorAll<HTMLElement>(".aster-editor-selection")];
  assert.deepEqual(selectionElements.map(element => ({ left: element.style.left, width: element.style.width })), [
    { left: "50px", width: "20px" },
    { left: "20px", width: "15px" },
  ]);
  assert.equal(requiredElement<HTMLElement>(line, ".aster-editor-caret").style.left, "35px");
  assert.equal(viewport.getPositionContentCoordinates(TextPosition.at(0, 3)).left, 35);
  dom.window.close();
});

test("EditorViewport resolves RTL pointer hits from the browser caret position", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("abc אבג");
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
    textDirection: EditorTextDirection.RightToLeft,
  });
  viewport.layout({ width: 300, height: 40 });
  const text = lineText(requiredLine(viewport.element, 0));
  assert.ok(text.firstChild);
  Object.defineProperty(dom.window.document, "caretPositionFromPoint", {
    configurable: true,
    value: () => ({ offsetNode: text.firstChild, offset: 5 }),
  });

  assert.deepEqual(viewport.getTargetAtClientPoint({ clientX: 170, clientY: 10 }), {
    kind: "text",
    position: TextPosition.at(0, 5),
  });
  dom.window.close();
});

test("EditorViewport announces cursor and selection changes through its live region", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("alpha\nbeta");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: fixedTextMeasurer(), selectionController: selections });

  const status = requiredElement(viewport.element, ".aster-editor-accessibility-status");
  assert.equal(status.getAttribute("aria-live"), "polite");
  assert.equal(status.textContent, "Line 1, column 1");
  selections.setSelections(TextSelectionSet.single(TextSelection.from(TextPosition.at(1, 1), TextPosition.at(1, 4))));
  assert.equal(status.textContent, "Line 2, column 5, 3 characters selected");
  selections.setSelections(TextSelectionSet.withPrimary([
    TextSelection.collapsedAt(TextPosition.at(0, 2)),
    TextSelection.from(TextPosition.at(1, 0), TextPosition.at(1, 2)),
  ], 1));
  assert.equal(status.textContent, "2 selections, 2 characters selected; primary at Line 2, column 3");
  dom.window.close();
});

test("EditorViewport accepts explicit accessibility status announcements", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("alpha");
  using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: fixedTextMeasurer() });

  viewport.announceAccessibilityStatus("  Saved  ");
  assert.equal(requiredElement(viewport.element, ".aster-editor-accessibility-status").textContent, "Saved");
  assert.throws(() => viewport.announceAccessibilityStatus("  "), /non-empty string/);
  dom.window.close();
});

test("EditorViewport projects indentation guides for visible logical rows only", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("    alpha\n  beta");
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(10),
    indentation: { tabSize: 2 },
  });
  viewport.layout({ width: 300, height: 40 });
  const firstGuides = requiredLine(viewport.element, 0).querySelectorAll<HTMLElement>(".aster-editor-indent-guide");
  assert.deepEqual([...firstGuides].map(guide => ({ level: guide.dataset.indentLevel, left: guide.style.left })), [
    { level: "1", left: "57px" },
    { level: "2", left: "77px" },
  ]);
  assert.equal(requiredLine(viewport.element, 1).querySelectorAll(".aster-editor-indent-guide").length, 1);
  dom.window.close();
});

test("EditorViewport renders a bounded minimap and maps a primary click to document scroll", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel(lines(200).join("\n"));
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
    minimap: EditorMinimap.On,
  });
  viewport.layout({ width: 300, height: 100 });
  const minimap = requiredElement<HTMLElement>(viewport.element, ".aster-editor-minimap");
  assert.equal(minimap.hidden, false);
  assert.equal(minimap.querySelectorAll(".aster-editor-minimap-row").length, 160);
  const overview = requiredElement<HTMLElement>(viewport.element, ".aster-editor-overview-ruler");
  assert.equal(overview.style.left, "234px");

  minimap.dispatchEvent(new dom.window.MouseEvent("pointerdown", {
    bubbles: true,
    cancelable: true,
    button: 0,
    clientY: 75,
  }));
  assert.equal(viewport.viewportLayout.scrollPosition.top, 2925);
  assert.equal(requiredElement<HTMLElement>(minimap, ".aster-editor-minimap-viewport").style.top, "73.125%");

  dom.window.document.dispatchEvent(new dom.window.MouseEvent("pointermove", {
    bubbles: true,
    cancelable: true,
    clientY: 100,
  }));
  assert.equal(viewport.viewportLayout.scrollPosition.top, 3900);
  assert.equal(minimap.classList.contains("dragging"), true);
  dom.window.document.dispatchEvent(new dom.window.MouseEvent("pointerup", { bubbles: true }));
  assert.equal(minimap.classList.contains("dragging"), false);
  dom.window.document.dispatchEvent(new dom.window.MouseEvent("pointermove", {
    bubbles: true,
    cancelable: true,
    clientY: 0,
  }));
  assert.equal(viewport.viewportLayout.scrollPosition.top, 3900);

  dom.window.close();
});

test("EditorViewport keeps minimaps out of embedded presentations", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("alpha");
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
    presentation: "embedded",
  });
  assert.equal(requiredElement<HTMLElement>(viewport.element, ".aster-editor-minimap").hidden, true);
  dom.window.close();
});

test("EditorViewport lets a direct host own its focus outline and omits active lines by default when embedded", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("alpha");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using embeddedViewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
    presentation: "embedded",
    focusOutlineOwner: "host",
    selectionController: selections,
  });
  embeddedViewport.layout({ width: 300, height: 40 });
  assert.equal(embeddedViewport.element.classList.contains("aster-editor-focus-owner-host"), true);
  assert.equal(embeddedViewport.element.classList.contains("aster-editor-focus-owner-editor"), false);
  assert.equal(embeddedViewport.element.querySelector(".aster-editor-line.active"), null);
  assert.ok(embeddedViewport.element.querySelector(".aster-editor-caret"));
  assert.throws(() => new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
    focusOutlineOwner: "unknown" as never,
  }), /Unknown Aster editor focus outline owner/);
  assert.throws(() => new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
    activeLineHighlight: "unknown" as never,
  }), /Unknown Aster editor active-line highlight/);
  dom.window.close();
});

test("EditorViewport rejects an unknown minimap mode", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("alpha");
  assert.throws(() => new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
    minimap: "invalid" as never,
  }), /Unknown Aster editor minimap mode/);
  dom.window.close();
});

test("Scrolling virtualizes rows while preserving overlapping DOM identity", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel(lines(100).join("\n"));
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    overscanLineCount: 2,
    textMeasurer: fixedTextMeasurer(),
  });
  viewport.layout({ width: 300, height: 100 });

  viewport.scrollTo({ left: 0, top: 400 });
  const line20 = requiredLine(viewport.element, 20);
  assert.deepEqual(viewport.viewportLayout.renderLines, {
    startLineIndex: 18,
    endLineIndexExclusive: 27,
  });
  assert.equal(
    requiredElement(viewport.element, ".aster-editor-lines").style.transform,
    "translate3d(0, 360px, 0)",
  );

  viewport.element.scrollTop = 420;
  viewport.element.dispatchEvent(new dom.window.Event("scroll"));

  assert.equal(viewport.viewportLayout.scrollPosition.top, 420);
  assert.deepEqual(viewport.viewportLayout.renderLines, {
    startLineIndex: 19,
    endLineIndexExclusive: 28,
  });
  assert.equal(requiredLine(viewport.element, 20), line20);
  assert.equal(viewport.element.querySelector('[data-line-index="18"]'), null);

  dom.window.close();
});

test("Soft wrapping virtualizes visual rows and maps DOM coordinates back to logical text", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("abcdef\ngh");
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(10, 24),
    lineWrapping: EditorLineWrapping.On,
  });

  viewport.layout({ width: 70, height: 40 });

  assert.equal(viewport.viewportLayout.contentSize.height, 80);
  assert.equal(viewport.viewportLayout.maximumScrollPosition.left, 0);
  assert.deepEqual(
    lineElements(viewport.element).map(line => ({
      lineIndex: line.dataset.lineIndex,
      logicalLineIndex: line.dataset.logicalLineIndex,
      number: lineNumber(line).textContent,
      text: lineText(line).textContent,
    })),
    [{
      lineIndex: "0",
      logicalLineIndex: "0",
      number: "1",
      text: "ab",
    }, {
      lineIndex: "1",
      logicalLineIndex: "0",
      number: "",
      text: "cd",
    }, {
      lineIndex: "2",
      logicalLineIndex: "0",
      number: "",
      text: "ef",
    }, {
      lineIndex: "3",
      logicalLineIndex: "1",
      number: "2",
      text: "gh",
    }],
  );
  assert.deepEqual(
    viewport.getPositionContentCoordinates(TextPosition.at(0, 3)),
    { left: 48, top: 20, height: 20 },
  );
  assert.deepEqual(viewport.getTargetAtClientPoint({ clientX: 50, clientY: 25 }), {
    kind: "text",
    position: TextPosition.at(0, 3),
  });

  viewport.layout({ width: 90, height: 40 });

  assert.equal(viewport.viewportLayout.contentSize.height, 60);
  assert.deepEqual(
    lineElements(viewport.element).map(line => lineText(line).textContent),
    ["abcd", "ef", "gh"],
  );

  dom.window.close();
});

test("Folding model removes folded physical rows from the viewport projection", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("header\nbody\nend\nafter");
  using folding = new EditorFoldingModel(model);
  using hiddenRanges = new EditorHiddenRangeModel(model, folding);
  folding.setRanges([{ startLineIndex: 0, endLineIndex: 2 }]);
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
    foldingModel: folding,
    hiddenRangeModel: hiddenRanges,
  });
  viewport.layout({ width: 300, height: 20 });
  const initialToggle = requiredElement<HTMLButtonElement>(viewport.element, ".aster-editor-fold-toggle");
  assert.equal(initialToggle.getAttribute("aria-expanded"), "true");
  assert.equal(initialToggle.textContent, "⌄");
  folding.setContainingLineCollapsed(0, true);

  assert.equal(viewport.viewportLayout.contentSize.height, 40);
  assert.deepEqual(lineElements(viewport.element).map(line => ({
    logicalLineIndex: line.dataset.logicalLineIndex,
    number: lineNumber(line).textContent,
    text: lineText(line).textContent,
  })), [{
    logicalLineIndex: "0",
    number: "1",
    text: "header",
  }, {
    logicalLineIndex: "3",
    number: "4",
    text: "after",
  }]);
  assert.deepEqual(viewport.getPositionContentCoordinates(TextPosition.at(1, 0)), {
    left: 36,
    top: 0,
    height: 20,
  });
  const collapsedToggle = requiredElement<HTMLButtonElement>(viewport.element, ".aster-editor-fold-toggle");
  assert.equal(collapsedToggle.getAttribute("aria-expanded"), "false");
  assert.equal(collapsedToggle.textContent, "›");

  dom.window.close();
});

test("Model edits refresh visible rows and clamp a shrinking document", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel(lines(100).join("\n"));
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    overscanLineCount: 2,
    textMeasurer: fixedTextMeasurer(),
  });
  viewport.layout({ width: 300, height: 100 });
  viewport.scrollTo({ left: 0, top: 400 });
  const line20 = requiredLine(viewport.element, 20);

  model.applyEdits([{
    range: TextRange.from(
      TextPosition.at(20, 0),
      TextPosition.at(20, model.getLineContent(20).length),
    ),
    text: "changed line",
  }]);

  assert.equal(requiredLine(viewport.element, 20), line20);
  assert.equal(lineText(line20).textContent, "changed line");
  assert.equal(viewport.viewportLayout.modelVersion, 2);

  const snapshot = model.createSnapshot();
  model.applyEdits([{
    range: TextRange.from(model.positionAt(0), model.positionAt(snapshot.length)),
    text: "first\nsecond",
  }]);

  assert.equal(viewport.element.scrollTop, 0);
  assert.equal(viewport.viewportLayout.scrollPosition.top, 0);
  assert.equal(viewport.viewportLayout.contentSize.height, 100);
  assert.deepEqual(
    lineElements(viewport.element).map(line => lineText(line).textContent),
    ["first", "second"],
  );

  dom.window.close();
});

test("Selection controller projects gutter state, ranges, and carets", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("abcd\nefgh\nij");
  using controller = new EditorSelectionController(
    model,
    TextSelectionSet.withPrimary([
      TextSelection.from(
        TextPosition.at(1, 3),
        TextPosition.at(0, 1),
      ),
      TextSelection.collapsedAt(TextPosition.at(2, 1)),
    ], 0),
  );
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(10, 24),
    selectionController: controller,
  });
  viewport.layout({ width: 200, height: 60 });

  const selectionElements = [
    ...viewport.element.querySelectorAll<HTMLElement>(
      ".aster-editor-selection",
    ),
  ];
  const caretElements = [
    ...viewport.element.querySelectorAll<HTMLElement>(
      ".aster-editor-caret",
    ),
  ];
  assert.deepEqual(
    selectionElements.map(element => ({
      lineIndex: element.parentElement?.parentElement?.dataset.lineIndex,
      left: element.style.left,
      width: element.style.width,
    })),
    [{
      lineIndex: "0",
      left: "48px",
      width: "40px",
    }, {
      lineIndex: "1",
      left: "38px",
      width: "30px",
    }],
  );
  assert.equal(caretElements.length, 2);
  assert.equal(caretElements[0]?.classList.contains("primary"), true);
  assert.equal(caretElements[0]?.style.left, "48px");
  assert.equal(
    lineNumber(requiredLine(viewport.element, 0))
      .classList.contains("active"),
    true,
  );

  controller.setSelections(TextSelectionSet.single(
    TextSelection.collapsedAt(TextPosition.at(1, 2)),
  ));

  assert.equal(
    viewport.element.querySelectorAll(
      ".aster-editor-selection",
    ).length,
    0,
  );
  assert.equal(
    requiredLine(viewport.element, 1)
      .querySelector<HTMLElement>(".aster-editor-caret")
      ?.style.left,
    "58px",
  );
  assert.equal(
    lineNumber(requiredLine(viewport.element, 1))
      .classList.contains("active"),
    true,
  );
  assert.equal(requiredLine(viewport.element, 1).classList.contains("active"), true);

  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 0)),
    text: "X\n",
  }]);

  assert.equal(controller.selections.primary.active.lineIndex, 2);
  assert.equal(
    requiredLine(viewport.element, 2)
      .querySelector<HTMLElement>(".aster-editor-caret")
      ?.style.left,
    "58px",
  );
  assert.equal(
    lineNumber(requiredLine(viewport.element, 2)).textContent,
    "3",
  );
  assert.equal(
    lineNumber(requiredLine(viewport.element, 2))
      .classList.contains("active"),
    true,
  );

  dom.window.close();
});

test("Measured content width, line height, and scroll stay synchronized", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel([
    "x".repeat(458),
    ...lines(29),
  ].join("\n"));
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(1, 24),
  });

  viewport.layout({ width: 200, height: 100 });
  viewport.scrollTo({ left: 1_000, top: 200 });
  viewport.setLineHeight(40);

  assert.deepEqual(viewport.viewportLayout.scrollPosition, {
    left: 300,
    top: 400,
  });
  assert.equal(viewport.element.scrollLeft, 300);
  assert.equal(viewport.element.scrollTop, 400);
  assert.equal(
    requiredElement(viewport.element, ".aster-editor-content").style.width,
    "500px",
  );
  for (const row of lineElements(viewport.element)) {
    assert.equal(row.style.height, "40px");
    assert.equal(row.style.lineHeight, "40px");
  }

  dom.window.close();
});

test("Line width indexing updates only affected model line groups", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("abcdef\nxx");
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(10, 24),
  });
  viewport.layout({ width: 50, height: 40 });
  viewport.scrollTo({ left: 1_000, top: 0 });
  assert.equal(viewport.viewportLayout.contentSize.width, 110);
  assert.equal(viewport.element.scrollLeft, 60);

  model.applyEdits([{
    range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 1)),
    text: "",
  }, {
    range: TextRange.from(TextPosition.at(0, 4), TextPosition.at(0, 5)),
    text: "",
  }]);

  assert.equal(model.getLineContent(0), "bcdf");
  assert.equal(viewport.viewportLayout.contentSize.width, 90);
  assert.equal(viewport.element.scrollLeft, 40);

  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(1, 2)),
    text: "\n0123456789",
  }]);

  assert.equal(viewport.viewportLayout.contentSize.width, 150);
  assert.equal(viewport.viewportLayout.maximumScrollPosition.left, 100);

  dom.window.close();
});

test("Font metric refresh rebuilds authoritative horizontal width", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  const measurer = fixedTextMeasurer(10, 20);
  using model = new TextModel("xxxx");
  using viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: measurer,
  });
  viewport.layout({ width: 50, height: 20 });
  assert.equal(viewport.viewportLayout.contentSize.width, 86);

  measurer.setCharacterWidth(20);
  viewport.refreshFontMetrics();

  assert.equal(viewport.viewportLayout.contentSize.width, 136);
  assert.equal(viewport.viewportLayout.maximumScrollPosition.left, 86);

  dom.window.close();
});

test("Viewport disposal removes DOM without owning the text model", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("alpha");
  const viewport = new EditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(),
  });

  viewport.dispose();
  assert.equal(container.childElementCount, 0);
  model.applyEdits([{
    range: TextRange.emptyAt(model.positionAt(5)),
    text: " editor",
  }]);
  assert.equal(model.getText(), "alpha editor");

  dom.window.close();
});

function requiredElement<T extends Element = HTMLElement>(
  container: ParentNode,
  selector: string,
): T {
  const element = container.querySelector<T>(selector);
  assert.ok(element, `Expected ${selector}`);
  return element;
}

function lineElements(container: ParentNode): HTMLDivElement[] {
  return [...container.querySelectorAll<HTMLDivElement>(
    ".aster-editor-line",
  )];
}

function requiredLine(container: ParentNode, lineIndex: number): HTMLDivElement {
  return requiredElement<HTMLDivElement>(
    container,
    `[data-line-index="${lineIndex}"]`,
  );
}

function lineText(line: Element | undefined): HTMLSpanElement {
  assert.ok(line);
  return requiredElement<HTMLSpanElement>(
    line,
    ".aster-editor-line-text",
  );
}

function lineNumber(line: Element | undefined): HTMLSpanElement {
  assert.ok(line);
  return requiredElement<HTMLSpanElement>(
    line,
    ".aster-editor-line-number",
  );
}

function lines(count: number): string[] {
  return Array.from({ length: count }, (_, index) => `line ${index}`);
}

function testRectangle(left: number, top: number, width: number): DOMRect {
  return { left, top, width, height: 20, right: left + width, bottom: top + 20, x: left, y: top, toJSON: () => ({}) } as DOMRect;
}

function fixedTextMeasurer(
  characterWidth = 8,
  horizontalPadding = 24,
): TestTextMeasurer {
  return new TestTextMeasurer(characterWidth, horizontalPadding);
}

class TestTextMeasurer implements TextMeasurer {
  private dirty = false;

  constructor(
    private characterWidth: number,
    readonly horizontalPadding: number,
  ) {}

  get contentLeftPadding(): number {
    return this.horizontalPadding / 2;
  }

  setCharacterWidth(characterWidth: number): void {
    this.characterWidth = characterWidth;
    this.dirty = true;
  }

  refresh(): boolean {
    const changed = this.dirty;
    this.dirty = false;
    return changed;
  }

  measureLineWidth(text: string): number {
    return [...text].length * this.characterWidth;
  }
}
