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
    "browser/editorPane.ts",
    "browser/editorWidget.ts",
    "browser/services/editorProfile.ts",
    "browser/widget/textEditorWidget.ts",
    "browser/media/editorWidget.css",
    "contrib/formatting/browser/formattingContribution.ts",
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
  const pane = readFileSync(join(gamaRoot, "browser/editorPane.ts"), "utf8");
  const editor = readFileSync(join(gamaRoot, "browser/editorWidget.ts"), "utf8");
  const formatting = readFileSync(join(gamaRoot, "contrib/formatting/browser/formattingContribution.ts"), "utf8");
  const widget = readFileSync(join(gamaRoot, "browser/widget/textEditorWidget.ts"), "utf8");
  const editorAll = readFileSync(join(gamaRoot, "editor.all.ts"), "utf8");
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
  assert.doesNotMatch(editor, /Session/u);
  assert.match(editorAll, /academicEditor\.contribution/u);
});

function directoryNames(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true })
    .filter(entry => entry.isDirectory())
    .map(entry => entry.name)
    .sort();
}

test("editor domains do not repeat their directory name in internal TypeScript symbols", () => {
  assertNoDomainPrefixedSymbols(join(editorRoot, "alpha"), "Alpha");
  assertNoDomainPrefixedSymbols(gamaRoot, "Gama");
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

function assertNoDomainPrefixedSymbols(root: string, domain: string): void {
  const expression = new RegExp(`\\b${domain}[A-Z][A-Za-z0-9_]*\\b`, "u");
  for (const file of collectFiles(root)) {
    if (!file.endsWith(".ts")) continue;
    assert.doesNotMatch(readFileSync(file, "utf8"), expression, relative(root, file));
  }
}
