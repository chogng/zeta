import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { EditorSelectionController } from "../../common/editorSelectionController.js";
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
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { AlphaEditorViewport } = await import(
  "../../browser/alphaEditorViewport.js"
);

test("AlphaEditorViewport projects the initial virtual line window", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel([
    "<strong>not markup</strong>",
    ...lines(99),
  ].join("\n"));
  using viewport = new AlphaEditorViewport({
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
      "--alpha-editor-gutter-width",
    ),
    "40px",
  );
  assert.equal(
    requiredElement(viewport.element, ".zeta-alpha-editor-content").style.height,
    "2000px",
  );

  dom.window.close();
});

test("Scrolling virtualizes rows while preserving overlapping DOM identity", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel(lines(100).join("\n"));
  using viewport = new AlphaEditorViewport({
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
    requiredElement(viewport.element, ".zeta-alpha-editor-lines").style.transform,
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

test("Model edits refresh visible rows and clamp a shrinking document", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel(lines(100).join("\n"));
  using viewport = new AlphaEditorViewport({
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
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: fixedTextMeasurer(10, 24),
    selectionController: controller,
  });
  viewport.layout({ width: 200, height: 60 });

  const selectionElements = [
    ...viewport.element.querySelectorAll<HTMLElement>(
      ".zeta-alpha-editor-selection",
    ),
  ];
  const caretElements = [
    ...viewport.element.querySelectorAll<HTMLElement>(
      ".zeta-alpha-editor-caret",
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
      ".zeta-alpha-editor-selection",
    ).length,
    0,
  );
  assert.equal(
    requiredLine(viewport.element, 1)
      .querySelector<HTMLElement>(".zeta-alpha-editor-caret")
      ?.style.left,
    "58px",
  );
  assert.equal(
    lineNumber(requiredLine(viewport.element, 1))
      .classList.contains("active"),
    true,
  );

  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 0)),
    text: "X\n",
  }]);

  assert.equal(controller.selections.primary.active.lineIndex, 2);
  assert.equal(
    requiredLine(viewport.element, 2)
      .querySelector<HTMLElement>(".zeta-alpha-editor-caret")
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
  using viewport = new AlphaEditorViewport({
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
    requiredElement(viewport.element, ".zeta-alpha-editor-content").style.width,
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
  using viewport = new AlphaEditorViewport({
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
  using viewport = new AlphaEditorViewport({
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
  const viewport = new AlphaEditorViewport({
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
    ".zeta-alpha-editor-line",
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
    ".zeta-alpha-editor-line-text",
  );
}

function lineNumber(line: Element | undefined): HTMLSpanElement {
  assert.ok(line);
  return requiredElement<HTMLSpanElement>(
    line,
    ".zeta-alpha-editor-line-number",
  );
}

function lines(count: number): string[] {
  return Array.from({ length: count }, (_, index) => `line ${index}`);
}

function fixedTextMeasurer(
  characterWidth = 8,
  horizontalPadding = 24,
): TestTextMeasurer {
  return new TestTextMeasurer(characterWidth, horizontalPadding);
}

class TestTextMeasurer implements AlphaTextMeasurer {
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
