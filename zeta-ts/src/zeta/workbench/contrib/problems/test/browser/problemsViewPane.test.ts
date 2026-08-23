import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Emitter } from "../../../../../base/common/event.js";
import { toDisposable } from "../../../../../base/common/lifecycle.js";
import { URI } from "../../../../../base/common/uri.js";
import { TextPosition, TextRange } from "../../../../../editor/common/core/text.js";
import { LanguageDiagnosticSeverity } from "../../../../../editor/common/languages/languageResults.js";
import { type ILanguageDiagnosticsService, type LanguageDiagnosticsPublisher, type LanguageDiagnosticSnapshot } from "../../../../../editor/common/services/languageDiagnosticsService.js";
import { type EditorInput, type EditorOpenOptions, type IEditorService } from "../../../../../workbench/services/editor/common/editorService.js";

test("ProblemsViewPane filters diagnostics and opens the selected range", async () => {
  const browser = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(browser);
  const main = URI.file("C:\\project\\src\\main.rs");
  const library = URI.file("C:\\project\\src\\lib.rs");
  const diagnostics = new FakeDiagnosticsService([
    snapshot(main, 3, LanguageDiagnosticSeverity.Error, "cannot find value", 1),
    snapshot(main, 3, LanguageDiagnosticSeverity.Warning, "unused import", 2),
    snapshot(library, 2, LanguageDiagnosticSeverity.Information, "consider simplifying", 4),
  ]);
  let opened: { readonly input: EditorInput; readonly options?: EditorOpenOptions } | undefined;
  let focusCount = 0;
  const editorService: IEditorService = {
    openEditor: async (input, options) => { opened = { input, options }; if (options?.preserveFocus !== true) focusCount += 1; },
    focusActiveEditor() { focusCount += 1; },
  };

  try {
    const { ProblemsViewPane } = await import("../../../../../workbench/contrib/problems/browser/problemsViewPane.js");
    using pane = new ProblemsViewPane(browser.window.document.body, { id: "zeta.problems", title: "Problems" }, diagnostics, editorService);
    browser.window.document.body.append(pane.element);
    const titleActions = pane.partTitleProjection?.actions;
    assert.ok(titleActions);
    browser.window.document.body.append(titleActions);
    const filterAction = titleActions.querySelector<HTMLButtonElement>("[data-action-id='zeta.problems.focusFilter'] button");
    assert.ok(filterAction);
    assert.ok(filterAction.querySelector("svg.zeta-icon"));
    assert.equal(pane.element.querySelectorAll(".zeta-problems-item").length, 3);
    assert.equal(pane.element.querySelector(".zeta-problems-status")?.textContent, "3 problems in the workspace.");

    const filter = pane.element.querySelector<HTMLInputElement>(".zeta-problems-filter")!;
    filterAction.click();
    assert.equal(browser.window.document.activeElement, filter);
    filter.value = "lib.rs";
    filter.dispatchEvent(new browser.window.Event("input", { bubbles: true }));
    assert.equal(pane.element.querySelectorAll(".zeta-problems-item").length, 1);
    assert.equal(pane.element.querySelector(".zeta-problems-status")?.textContent, "1 of 3 problems shown.");

    filter.value = "";
    filter.dispatchEvent(new browser.window.Event("input", { bubbles: true }));
    pane.element.querySelector<HTMLButtonElement>(".zeta-problems-severity.warning")!.click();
    assert.equal(pane.element.querySelectorAll(".zeta-problems-item").length, 2);
    assert.equal(pane.element.querySelector(".zeta-problems-severity.warning")?.classList.contains("checked"), false);
    assert.equal(pane.element.querySelector(".zeta-problems-severity.warning")?.getAttribute("aria-pressed"), "false");

    pane.element.querySelector<HTMLButtonElement>(".zeta-problems-item.error .zeta-problems-item-button")!.click();
    await Promise.resolve();
    assert.equal(opened?.input.resource.toString(), main.toString());
    assert.deepEqual(opened?.options?.selection, TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 4)));
    assert.equal(focusCount, 1);

    diagnostics.replace([]);
    assert.equal(pane.element.querySelectorAll(".zeta-problems-item").length, 0);
    assert.equal(pane.element.querySelector(".zeta-problems-status")?.textContent, "No problems have been detected in the workspace.");
  } finally {
    diagnostics.dispose();
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
    browser.window.close();
  }
});

class FakeDiagnosticsService implements ILanguageDiagnosticsService {
  private readonly emitter = new Emitter<URI>();
  readonly onDidChangeDiagnostics = this.emitter.event;

  constructor(private snapshots: readonly LanguageDiagnosticSnapshot[]) {}

  acquire() { return toDisposable(() => undefined); }
  createPublisher(): LanguageDiagnosticsPublisher { throw new Error("Problems view must not publish diagnostics"); }
  getDiagnostics(resource: URI) { return this.snapshots.find(snapshot => snapshot.resource.toString() === resource.toString()); }
  getAllDiagnostics() { return this.snapshots; }
  replace(snapshots: readonly LanguageDiagnosticSnapshot[]): void {
    this.snapshots = snapshots;
    this.emitter.fire(URI.file("C:\\project\\src\\main.rs"));
  }
  dispose(): void { this.emitter.dispose(); }
}

function snapshot(resource: URI, revision: number, severity: LanguageDiagnosticSeverity, message: string, lineNumber: number): LanguageDiagnosticSnapshot {
  return Object.freeze({
    resource,
    revision,
    diagnostics: Object.freeze([Object.freeze({
      range: TextRange.from(TextPosition.at(lineNumber, 0), TextPosition.at(lineNumber, 4)),
      severity,
      message,
      source: "fixture",
    })]),
  });
}

function installDomGlobals(browser: JSDOM): readonly string[] {
  const globals = {
    window: browser.window,
    document: browser.window.document,
    Node: browser.window.Node,
    Element: browser.window.Element,
    HTMLElement: browser.window.HTMLElement,
    Event: browser.window.Event,
    MouseEvent: browser.window.MouseEvent,
    navigator: browser.window.navigator,
  };
  for (const [name, value] of Object.entries(globals)) Object.defineProperty(globalThis, name, { configurable: true, value });
  return Object.keys(globals);
}
