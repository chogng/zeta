import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { URI } from "../../../../../base/common/uri.js";
import { ServiceCollection } from "../../../../../platform/instantiation/common/instantiation.js";
import type { IEditorPart as IEditorPartContract } from "../../../../browser/parts/editor/editorPart.js";
import { BrowserUntitledTextEditorService } from "../../browser/browserUntitledTextEditorService.js";
import { IUntitledTextEditorService } from "../../common/untitledTextEditorService.js";
import { CommandService } from "../../../commands/common/commandService.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { IEditorPart } = await import("../../../../browser/parts/editor/editorPart.js");
const { NewUntitledTextEditorCommandId } = await import("../../../../browser/parts/editor/editorActions.js");

test.after(() => browserEnvironment.window.close());

test("untitled service creates stable virtual editor identities", () => {
  using service = new BrowserUntitledTextEditorService();
  const first = service.create();
  const second = service.create({ initialText: "draft", languageId: "typescript" });

  assert.equal(first.resource.toString(), "untitled:/Untitled-1");
  assert.equal(first.label, "Untitled-1");
  assert.equal(first.initialText, "");
  assert.equal(second.resource.toString(), "untitled:/Untitled-2");
  assert.equal(second.initialText, "draft");
  assert.equal(second.languageId, "typescript");
  assert.equal(service.get(first.resource), first);
  assert.equal(service.get(URI.file("C:\\project\\main.ts")), undefined);
  assert.equal(service.isUntitled(first.resource), true);
  assert.equal(service.isUntitled(URI.file("C:\\project\\main.ts")), false);
});

test("New Untitled Text Editor opens a compatible text editor input", async () => {
  using untitled = new BrowserUntitledTextEditorService();
  const opened: Array<{ readonly resource: URI; readonly label?: string; readonly initialText?: string }> = [];
  const editorPart = { openEditor: async (input: typeof opened[number]) => { opened.push(input); } } as unknown as IEditorPartContract;
  const services = new ServiceCollection();
  services.set(IUntitledTextEditorService, untitled);
  services.set(IEditorPart, editorPart);
  using commands = new CommandService(services);

  await commands.executeCommand(NewUntitledTextEditorCommandId);

  assert.equal(opened.length, 1);
  assert.equal(opened[0]?.resource.toString(), "untitled:/Untitled-1");
  assert.equal(opened[0]?.label, "Untitled-1");
  assert.equal(opened[0]?.initialText, "");
});
