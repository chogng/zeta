import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import test from "node:test";
import { findDesktopRoot } from "./testPaths.js";

const desktopRoot = findDesktopRoot(import.meta.dirname);
const editorRoot = resolve(desktopRoot, "src/zeta/editor");

test("Editor keeps explicit feature files without index barrels", () => {
  const indexFiles = collectFiles(editorRoot).filter(file => file.endsWith("\\index.ts") || file.endsWith("/index.ts"));
  assert.deepEqual(indexFiles, []);
});

test("Editor synchronous layers do not import Workbench, Electron, or generated DTOs", () => {
  const protectedDirectories = [
    "common/core",
    "common/model",
    "common/cursor",
    "common/commands",
    "common/viewLayout",
    "common/viewModel",
  ];
  for (const directory of protectedDirectories) {
    for (const file of collectFiles(join(editorRoot, directory))) {
      if (!file.endsWith(".ts")) continue;
      const source = readFileSync(file, "utf8");
      assert.doesNotMatch(source, /from\s+["'][^"']*(?:workbench|electron|generated)[^"']*["']/u, relative(editorRoot, file));
    }
  }
});

test("Flat editor layout keeps both engine owners and product bundles", () => {
  const requiredFiles = [
    "browser/editorPart.ts",
    "browser/browserEditorPart.ts",
    "browser/view/editorViewport.ts",
    "browser/input/textInputController.ts",
    "browser/services/rustDiffComputationService.ts",
    "common/core/position.ts",
    "common/model/decorationCollection.ts",
    "common/model/textModel.ts",
    "common/cursor/editorSelectionController.ts",
    "common/services/languageService.ts",
    "contrib/gotoError/browser/gotoError.ts",
    "contrib/indentation/browser/indentation.ts",
    "contrib/gpu/browser/gpuRenderer.ts",
    "contrib/longLinesHelper/browser/longLinesHelper.ts",
    "contrib/tokenization/common/tokenizationTextModelPart.ts",
    "contrib/semanticTokens/common/semanticTokens.ts",
    "browser/codeEditorPane.ts",
    "browser/documentEditorPane.ts",
    "common/model/documentModel.ts",
    "common/model/documentTransaction.ts",
    "contrib/academic/browser/academicEditor.contribution.ts",
    "editor.code.all.ts",
    "editor.academic.all.ts",
    "editor.all.ts",
    "editor.api.ts",
    "text-engine-implementation-ledger.md",
  ];
  for (const file of requiredFiles) assert.equal(statSafe(join(editorRoot, file)), true, file);

  const removedLegacyNames = [
    "browser/editorSession.ts",
    "browser/browserEditorSession.ts",
    "common/model/decoration.ts",
    "contrib/gotoError/browser/gotoErrorController.ts",
    "contrib/indentation/browser/indentationGuides.ts",
  ];
  for (const file of removedLegacyNames) assert.equal(statSafe(join(editorRoot, file)), false, file);
  assert.equal(existsSync(join(editorRoot, "alpha")), false, "alpha directory");
  assert.equal(existsSync(join(editorRoot, "gama")), false, "gama directory");
});

test("Text engine PieceTree tests follow VS Code's common model layout", () => {
  assert.equal(statSafe(join(editorRoot, "test/common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.test.ts")), true);
  assert.equal(statSafe(join(editorRoot, "test/common/pieceTreeTextBuffer.test.ts")), false);
});

test("Product entries statically select their editor contribution bundles", () => {
  const codeBundle = readFileSync(join(editorRoot, "editor.code.all.ts"), "utf8");
  const academicBundle = readFileSync(join(editorRoot, "editor.academic.all.ts"), "utf8");
  const completeBundle = readFileSync(join(editorRoot, "editor.all.ts"), "utf8");
  assert.match(codeBundle, /contrib\/editor\.contribution/u);
  assert.doesNotMatch(codeBundle, /contrib\/academic/u);
  assert.match(academicBundle, /contrib\/academic\/browser\/academicEditor\.contribution/u);
  assert.doesNotMatch(academicBundle, /contrib\/editor\.contribution/u);
  assert.match(completeBundle, /editor\.code\.all/u);
  assert.match(completeBundle, /editor\.academic\.all/u);

  const codeEntry = readFileSync(resolve(editorRoot, "../code/browser/workbench/workbench-code.ts"), "utf8");
  const academicEntry = readFileSync(resolve(editorRoot, "../code/browser/workbench/workbench-academic.ts"), "utf8");
  const electronCodeEntry = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/workbench-code.ts"), "utf8");
  const electronAcademicEntry = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/workbench-academic.ts"), "utf8");
  assert.match(codeEntry, /editor\/editor\.code\.all/u);
  assert.doesNotMatch(codeEntry, /editor\/editor\.academic\.all/u);
  assert.match(academicEntry, /editor\/editor\.academic\.all/u);
  assert.doesNotMatch(academicEntry, /editor\/editor\.code\.all/u);
  assert.match(electronCodeEntry, /editor\/editor\.code\.all/u);
  assert.doesNotMatch(electronCodeEntry, /editor\/editor\.academic\.all/u);
  assert.match(electronAcademicEntry, /editor\/editor\.academic\.all/u);
  assert.doesNotMatch(electronAcademicEntry, /editor\/editor\.code\.all/u);
});

test("Editor engines delegate optional feature composition to product bundles", () => {
  const textHost = readFileSync(join(editorRoot, "browser/editorPart.ts"), "utf8");
  const textContribution = readFileSync(join(editorRoot, "contrib/codeEditorPart.contribution.ts"), "utf8");
  const findContribution = readFileSync(join(editorRoot, "contrib/find/browser/find.contribution.ts"), "utf8");
  const documentHost = readFileSync(join(editorRoot, "browser/editorWidget.ts"), "utf8");
  const documentContribution = readFileSync(join(editorRoot, "contrib/documentEditor.contribution.ts"), "utf8");
  const codePaneContribution = readFileSync(join(editorRoot, "contrib/editor.contribution.ts"), "utf8");
  const academicPaneContribution = readFileSync(join(editorRoot, "contrib/academic/browser/academicEditor.contribution.ts"), "utf8");
  const codeBundle = readFileSync(join(editorRoot, "editor.code.all.ts"), "utf8");
  const academicBundle = readFileSync(join(editorRoot, "editor.academic.all.ts"), "utf8");
  assert.doesNotMatch(textHost, /from\s+["'][^"']*\/contrib\/(?:find|folding|hover|format|rename|codeAction|collaboration|formatting)\//u);
  assert.match(textHost, /registerEditorPartFactory/u);
  assert.match(textContribution, /registerEditorPartFactory/u);
  assert.doesNotMatch(textContribution, /FindController/u);
  assert.match(findContribution, /registerEditorContribution/u);
  assert.match(codeBundle, /find\/browser\/find\.contribution/u);
  assert.match(academicBundle, /find\/browser\/find\.contribution/u);
  assert.doesNotMatch(codePaneContribution, /codeEditorPart\.contribution/u);
  assert.doesNotMatch(documentHost, /from\s+["'][^"']*\/contrib\/(?:formatting|collaboration)\/browser\//u);
  assert.match(documentHost, /getEditorContributions/u);
  assert.doesNotMatch(documentHost, /registerDocumentEditorContributionFactory/u);
  assert.match(documentContribution, /registerEditorContribution/u);
  assert.match(documentContribution, /FormattingContribution/u);
  assert.match(documentContribution, /CollaborationContribution/u);
  assert.doesNotMatch(academicPaneContribution, /codeEditorPart\.contribution/u);
  assert.doesNotMatch(academicPaneContribution, /documentEditor\.contribution/u);
  assert.match(codeBundle, /codeEditorPart\.contribution/u);
  assert.match(academicBundle, /codeEditorPart\.contribution/u);
  assert.match(academicBundle, /documentEditor\.contribution/u);
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
    return statSync(file).isFile();
  } catch {
    return false;
  }
}
