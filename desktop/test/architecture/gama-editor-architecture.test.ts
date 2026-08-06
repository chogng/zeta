import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import test from "node:test";

const editorRoot = resolve(import.meta.dirname, "../../../..", "src/zeta/editor");
const gamaRoot = join(editorRoot, "gama");

test("editor exposes Alpha and Gama as its only editor domains", () => {
  assert.deepEqual(directoryNames(editorRoot), ["alpha", "gama"]);
  assert.equal(statSafe(join(editorRoot, "common")), false);
  assert.equal(statSafe(join(editorRoot, "core")), false);
  assert.equal(statSafe(join(editorRoot, "textEditorWidget")), false);
  assert.equal(statSafe(gamaRoot), true);
  assert.deepEqual(directoryNames(gamaRoot), ["browser", "common", "contrib", "test"]);
  assert.deepEqual(directoryNames(join(gamaRoot, "common")), ["commands", "core", "model", "services"]);
  assert.deepEqual(directoryNames(join(gamaRoot, "browser")), ["media", "services", "widget"]);
  assert.equal(statSafe(join(gamaRoot, "contrib", "academic")), true);
  assert.equal(statSafe(join(gamaRoot, "academic")), false);
  assert.equal(statSafe(join(gamaRoot, "editor.all.ts")), true);
  assert.deepEqual(collectFiles(gamaRoot).filter(file => /[\\/]index\.ts$/u.test(file)), []);
});

test("Gama follows VS Code editor common/browser/contrib ownership", () => {
  for (const file of [
    "common/core/documentSelection.ts",
    "common/model/documentModel.ts",
    "common/services/documentModelService.ts",
    "common/commands/documentCommands.ts",
    "browser/services/documentWorkingCopy.ts",
    "browser/services/browserDocumentModelService.ts",
    "browser/editorInput.ts",
    "browser/gamaEditorPane.ts",
    "browser/gamaEditorSession.ts",
    "browser/services/gamaEditorProfile.ts",
    "browser/widget/textEditorWidget.ts",
    "browser/media/gamaEditorSession.css",
    "contrib/academic/browser/profile.ts",
    "contrib/academic/browser/academicEditor.contribution.ts",
  ]) assert.equal(statSafe(join(gamaRoot, file)), true, file);
  assert.equal(statSafe(join(gamaRoot, "contrib", "editor.contribution.ts")), false);
  for (const file of collectFiles(join(gamaRoot, "common"))) {
    if (!file.endsWith(".ts")) continue;
    const source = readFileSync(file, "utf8");
    assert.doesNotMatch(source, /from\s+["'][^"']*(?:editor\/alpha|workbench|electron)[^"']*["']/u, relative(gamaRoot, file));
  }
});

test("Gama keeps textBlock semantics and uses Alpha only through the embedded-editor seam", () => {
  const schema = readFileSync(join(gamaRoot, "common/model/documentSchema.ts"), "utf8");
  const pane = readFileSync(join(gamaRoot, "browser/gamaEditorPane.ts"), "utf8");
  const session = readFileSync(join(gamaRoot, "browser/gamaEditorSession.ts"), "utf8");
  const widget = readFileSync(join(gamaRoot, "browser/widget/textEditorWidget.ts"), "utf8");
  const editorAll = readFileSync(join(gamaRoot, "editor.all.ts"), "utf8");
  assert.match(schema, /textBlock:/u);
  assert.doesNotMatch(schema, /codeBlock/u);
  assert.match(pane, /export class GamaEditorPane/u);
  assert.match(pane, /implements IEditorPane/u);
  assert.match(pane, /BrowserDocumentModelService/u);
  assert.match(session, /export class GamaEditorSession/u);
  assert.match(session, /IDocumentModelService/u);
  assert.match(session, /DocumentModelReference/u);
  assert.match(widget, /export class TextEditorWidget/u);
  assert.match(widget, /IEmbeddedTextEditorFactory/u);
  assert.match(session, /new TextEditorWidget\(/u);
  assert.doesNotMatch(widget, /AlphaEmbeddedTextEditorFactory/u);
  assert.match(editorAll, /academicEditor\.contribution/u);
});

function directoryNames(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true })
    .filter(entry => entry.isDirectory())
    .map(entry => entry.name)
    .sort();
}

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
