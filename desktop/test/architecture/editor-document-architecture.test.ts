import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import test from "node:test";
import { findDesktopRoot } from "./testPaths.js";

const editorRoot = resolve(findDesktopRoot(import.meta.dirname), "src/zeta/editor");

test("editor exposes one flat VS Code-shaped domain for both engines", () => {
  assert.deepEqual(directoryNames(editorRoot), ["browser", "common", "contrib", "test"]);
  assert.deepEqual(directoryNames(join(editorRoot, "common")), ["commands", "core", "cursor", "diff", "languages", "model", "services", "tokens", "viewLayout", "viewModel"]);
  assert.deepEqual(directoryNames(join(editorRoot, "browser")), ["input", "language", "media", "services", "view", "widget"]);
  assert.equal(statSafe(join(editorRoot, "contrib", "academic")), true);
  assert.equal(statSafe(join(editorRoot, "alpha")), false);
  assert.equal(statSafe(join(editorRoot, "gama")), false);
  assert.equal(statSafe(join(editorRoot, "editor.academic.all.ts")), true);
  assert.deepEqual(collectFiles(editorRoot).filter(file => /[\\/]index\.ts$/u.test(file)), []);
});

test("document editing follows VS Code editor common/browser/contrib ownership", () => {
  for (const file of [
    "common/core/documentSelection.ts",
    "common/model/documentModel.ts",
    "common/services/documentModelService.ts",
    "common/commands/documentCommands.ts",
    "browser/services/documentWorkingCopy.ts",
    "browser/services/browserDocumentModelService.ts",
    "browser/documentEditorInput.ts",
    "browser/documentEditorPane.ts",
    "browser/editorWidget.ts",
    "browser/services/editorProfile.ts",
    "browser/widget/textEditorWidget.ts",
    "browser/media/editorWidget.css",
    "contrib/clipboard/browser/htmlDocumentFragment.ts",
    "contrib/formatting/browser/formattingContribution.ts",
    "contrib/collaboration/common/protocol.ts",
    "contrib/collaboration/common/controller.ts",
    "contrib/collaboration/browser/collaborationContribution.ts",
    "common/services/documentCollaborationService.ts",
    "browser/services/appServerDocumentCollaborationService.ts",
    "contrib/academic/browser/profile.ts",
    "contrib/academic/browser/academicEditor.contribution.ts",
  ]) assert.equal(statSafe(join(editorRoot, file)), true, file);
  assert.equal(statSafe(join(editorRoot, "contrib", "collaboration", "common", "session.ts")), false);
  for (const file of collectFiles(join(editorRoot, "common"))) {
    if (!file.endsWith(".ts")) continue;
    const source = readFileSync(file, "utf8");
    assert.doesNotMatch(source, /from\s+["'][^"']*(?:workbench|electron)[^"']*["']/u, relative(editorRoot, file));
  }
});

test("document editing keeps textBlock semantics behind the embedded-editor seam", () => {
  const schema = readFileSync(join(editorRoot, "common/model/documentSchema.ts"), "utf8");
  const pane = readFileSync(join(editorRoot, "browser/documentEditorPane.ts"), "utf8");
  const editor = readFileSync(join(editorRoot, "browser/editorWidget.ts"), "utf8");
  const formatting = readFileSync(join(editorRoot, "contrib/formatting/browser/formattingContribution.ts"), "utf8");
  const widget = readFileSync(join(editorRoot, "browser/widget/textEditorWidget.ts"), "utf8");
  const editorAll = readFileSync(join(editorRoot, "editor.academic.all.ts"), "utf8");
  assert.match(schema, /textBlock:/u);
  assert.doesNotMatch(schema, /codeBlock/u);
  assert.match(pane, /export class EditorPane/u);
  assert.match(pane, /implements IEditorPane/u);
  assert.match(pane, /BrowserDocumentModelService/u);
  assert.match(editor, /export class EditorWidget/u);
  assert.match(editor, /IDocumentModelService/u);
  assert.match(editor, /DocumentModelReference/u);
  assert.match(widget, /export class TextEditorWidget/u);
  assert.match(widget, /IEmbeddedTextEditorFactory/u);
  assert.match(editor, /new TextEditorWidget\(/u);
  assert.doesNotMatch(widget, /editor\/alpha\/browser\/embeddedTextEditor/u);
  assert.match(formatting, /new ToolBar\(/u);
  const collaborationService = readFileSync(join(editorRoot, "common/services/documentCollaborationService.ts"), "utf8");
  const collaborationWidget = readFileSync(join(editorRoot, "browser/editorWidget.ts"), "utf8");
  assert.match(collaborationService, /export interface IDocumentCollaborationService/u);
  assert.doesNotMatch(collaborationService, /from\s+["'][^"']*(?:platform|workbench|electron|generated)[^"']*["']/u);
  assert.match(collaborationWidget, /CollaborationContribution/u);
  assert.doesNotMatch(collaborationWidget, /AppServerDocumentCollaborationService/u);
  assert.doesNotMatch(editor, /Session/u);
  assert.match(editorAll, /academicEditor\.contribution/u);
});

function directoryNames(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true })
    .filter(entry => entry.isDirectory())
    .map(entry => entry.name)
    .sort();
}

test("flat editor paths do not reintroduce Alpha or Gama directories", () => {
  for (const file of collectFiles(editorRoot)) {
    assert.doesNotMatch(relative(editorRoot, file), /(?:^|[\\/])(?:alpha|gama)(?:[\\/]|$)/u);
  }
});

function collectFiles(directory: string): string[] {
  const result: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const file = join(directory, entry.name);
    if (entry.isDirectory()) result.push(...collectFiles(file));
    else result.push(file);
  }
  return result;
}

function statSafe(file: string): boolean {
  try {
    return statSync(file).isDirectory() || statSync(file).isFile();
  } catch {
    return false;
  }
}
