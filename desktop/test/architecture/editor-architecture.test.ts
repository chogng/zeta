import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import test from "node:test";
import { findDesktopRoot } from "./testPaths.js";

const desktopRoot = findDesktopRoot(import.meta.dirname);
const editorRoot = resolve(desktopRoot, "src/zeta/editor");
const workbenchRoot = resolve(desktopRoot, "src/zeta/workbench");

test("Editor keeps explicit feature files without index barrels", () => {
  const indexFiles = collectFiles(editorRoot).filter(file => file.endsWith("\\index.ts") || file.endsWith("/index.ts"));
  assert.deepEqual(indexFiles, []);
});

test("Editor production code does not depend on Workbench or generated transport DTOs", () => {
  for (const file of collectFiles(editorRoot)) {
    if (!file.endsWith(".ts") || file.includes(`${join("editor", "test")}`)) continue;
    const source = readFileSync(file, "utf8");
    assert.doesNotMatch(source, /from\s+["'][^"']*workbench[^"']*["']|import\s+["'][^"']*workbench[^"']*["']/u, relative(editorRoot, file));
    assert.doesNotMatch(source, /from\s+["'][^"']*generated\/app-server[^"']*["']/u, relative(editorRoot, file));
  }
});

test("Editor synchronous layers do not import Electron or generated DTOs", () => {
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
      assert.doesNotMatch(source, /from\s+["'][^"']*(?:electron|generated)[^"']*["']/u, relative(editorRoot, file));
    }
  }
});

test("Flat editor layout keeps both engine owners and product bundles", () => {
  const requiredFiles = [
    "browser/editorPart.ts",
    "browser/view/editorViewport.ts",
    "browser/input/textInputController.ts",
    "browser/services/rustDiffComputationService.ts",
    "common/core/position.ts",
    "common/model/decorationCollection.ts",
    "common/model/textModel.ts",
    "common/cursor/editorSelectionController.ts",
    "common/services/languageService.ts",
    "contrib/gotoError/browser/gotoError.ts",
    "browser/view/indentationGuides.ts",
    "browser/view/gpuMinimapRenderer.ts",
    "browser/view/lineWidthIndex.ts",
    "contrib/tokenization/common/tokenizationTextModelPart.ts",
    "contrib/semanticTokens/common/semanticTokens.ts",
    "common/editorResource.ts",
    "browser/widget/embeddedTextEditor.ts",
    "common/model/documentModel.ts",
    "common/model/documentTransaction.ts",
    "contrib/academic/common/schema.ts",
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
    "contrib/indentation/browser/indentation.ts",
  ];
  for (const file of removedLegacyNames) assert.equal(statSafe(join(editorRoot, file)), false, file);
  assert.equal(existsSync(join(editorRoot, "alpha")), false, "alpha directory");
  assert.equal(existsSync(join(editorRoot, "gama")), false, "gama directory");
});

test("Aster source does not retain retired engine compatibility identifiers", () => {
  const sourceRoot = resolve(desktopRoot, "src/zeta");
  const legacyCompatibilityPattern = /zeta-(?:alpha|gama)|zeta\.editor\.(?:alpha|gama)|application\/(?:x-|vnd\.)?zeta(?:[-.](?:alpha|gama))|--(?:alpha|gama)-editor-|(?:ALPHA|GAMA)_EDITOR_ID/u;
  for (const file of collectFiles(sourceRoot)) {
    if (!/\.(?:css|md|ts)$/u.test(file)) continue;
    assert.doesNotMatch(readFileSync(file, "utf8"), legacyCompatibilityPattern, relative(sourceRoot, file));
  }
});

test("Aster owns its public protocol and DOM vocabulary without renaming the editor domain", () => {
  const api = readFileSync(join(editorRoot, "editor.api.ts"), "utf8");
  const codeInput = readFileSync(join(workbenchRoot, "contrib/codeEditor/browser/codeEditorInput.ts"), "utf8");
  const documentInput = readFileSync(join(workbenchRoot, "contrib/documentEditor/browser/documentEditorInput.ts"), "utf8");
  const diffInput = readFileSync(join(workbenchRoot, "contrib/codeEditor/browser/diffEditorInput.ts"), "utf8");
  const viewport = readFileSync(join(editorRoot, "browser/view/editorViewport.ts"), "utf8");
  assert.match(api, /Stable DOM-free Aster API/u);
  assert.match(codeInput, /aster\.editor\.code/u);
  assert.match(documentInput, /aster\.editor\.document/u);
  assert.match(diffInput, /aster\.editor\.diff/u);
  assert.match(diffInput, /application\/vnd\.aster\.editor-diff/u);
  assert.match(viewport, /aster-editor/u);
  assert.equal(existsSync(resolve(editorRoot, "../aster")), false, "parallel aster directory");
});

test("Text engine PieceTree tests follow VS Code's common model layout", () => {
  assert.equal(statSafe(join(editorRoot, "test/common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.test.ts")), true);
  assert.equal(statSafe(join(editorRoot, "test/common/pieceTreeTextBuffer.test.ts")), false);
});

test("Build modes statically select their Aster contribution bundles behind one Workbench entry", () => {
  const codeBundle = readFileSync(join(editorRoot, "editor.code.all.ts"), "utf8");
  const academicBundle = readFileSync(join(editorRoot, "editor.academic.all.ts"), "utf8");
  const standardBundle = readFileSync(join(editorRoot, "editor.all.ts"), "utf8");
  assert.match(codeBundle, /editor\.all/u);
  assert.doesNotMatch(codeBundle, /contrib\//u);
  assert.doesNotMatch(codeBundle, /contrib\/academic/u);
  assert.match(academicBundle, /editor\.all/u);
  assert.match(academicBundle, /contrib\/documentEditor\.contribution/u);
  assert.doesNotMatch(academicBundle, /workbench|academicEditor\.contribution/u);
  assert.match(standardBundle, /contrib\/codeEditorPart\.contribution/u);
  assert.doesNotMatch(standardBundle, /editor\.(?:code|academic)\.all/u);

  const browserEntry = readFileSync(resolve(editorRoot, "../code/browser/workbench/workbench.ts"), "utf8");
  const electronEntry = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/workbench.ts"), "utf8");
  const codeContribution = readFileSync(resolve(editorRoot, "../code/browser/workbench/modes/code.contribution.ts"), "utf8");
  const academicContribution = readFileSync(resolve(editorRoot, "../code/browser/workbench/modes/academic.contribution.ts"), "utf8");
  const electronCodeMode = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/modes/code.ts"), "utf8");
  const electronAcademicMode = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/modes/academic.ts"), "utf8");
  for (const entry of [browserEntry, electronEntry]) {
    assert.match(entry, /__ZETA_PRODUCT__/u);
    assert.match(entry, /modes\/code/u);
    assert.match(entry, /modes\/academic/u);
    assert.doesNotMatch(entry, /editor\/editor\.(?:code|academic)\.all/u);
  }
  assert.match(codeContribution, /editor\/editor\.code\.all/u);
  assert.match(codeContribution, /workbench\/contrib\/codeEditor\/browser\/codeEditor\.contribution/u);
  assert.match(codeContribution, /workbench\/contrib\/tasks\/browser\/tasks\.contribution/u);
  assert.match(codeContribution, /workbench\/contrib\/testing\/browser\/testing\.contribution/u);
  assert.match(codeContribution, /workbench\/contrib\/debug\/browser\/debug\.contribution/u);
  assert.match(codeContribution, /codeWorkbenchServices/u);
  assert.doesNotMatch(codeContribution, /editor\/editor\.academic\.all/u);
  assert.match(academicContribution, /editor\/editor\.academic\.all/u);
  assert.match(academicContribution, /workbench\/contrib\/academic\/browser\/academicEditor\.contribution/u);
  assert.doesNotMatch(academicContribution, /workbench\/contrib\/(?:tasks|testing|debug)/u);
  assert.doesNotMatch(academicContribution, /workbench\/contrib\/extensionHost/u);
  assert.doesNotMatch(academicContribution, /editor\/editor\.code\.all/u);
  assert.match(electronCodeMode, /browser\/workbench\/modes\/code\.contribution/u);
  assert.match(electronAcademicMode, /browser\/workbench\/modes\/academic\.contribution/u);
});

test("Code services are installed by product-selected service registrations rather than UI contributions", () => {
  const workbench = readFileSync(join(workbenchRoot, "browser/workbench.ts"), "utf8");
  const productServices = readFileSync(resolve(workbenchRoot, "../code/browser/workbench/codeWorkbenchServices.ts"), "utf8");
  const tasks = readFileSync(join(workbenchRoot, "services/tasks/browser/taskServiceRegistration.ts"), "utf8");
  const testing = readFileSync(join(workbenchRoot, "services/testing/browser/testingServiceRegistration.ts"), "utf8");
  const debug = readFileSync(join(workbenchRoot, "services/debug/browser/debugServiceRegistration.ts"), "utf8");
  assert.doesNotMatch(workbench, /services\/(?:tasks|testing|debug)\/(?:browser|common)/u);
  assert.doesNotMatch(workbench, /product\.id\s*===\s*["']code["']/u);
  assert.match(workbench, /installWorkbenchServiceContributions/u);
  assert.match(productServices, /taskServiceRegistration/u);
  assert.match(productServices, /testingServiceRegistration/u);
  assert.match(productServices, /debugServiceRegistration/u);
  assert.match(productServices, /extensionHostServiceRegistration/u);
  assert.match(productServices, /symbolIndexServiceRegistration/u);
  for (const registration of [tasks, testing, debug]) assert.match(registration, /registerWorkbenchServiceContribution/u);
  for (const contribution of ["tasks", "testing", "debug"]) assert.doesNotMatch(readFileSync(join(workbenchRoot, `contrib/${contribution}/browser/${contribution}.contribution.ts`), "utf8"), /registerWorkbenchServiceContribution/u);
});

test("Debug transport is contributed by Code product hosts only", () => {
  const browserCode = readFileSync(resolve(editorRoot, "../code/browser/workbench/modes/code.ts"), "utf8");
  const browserAcademic = readFileSync(resolve(editorRoot, "../code/browser/workbench/modes/academic.ts"), "utf8");
  const electronCode = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/modes/code.ts"), "utf8");
  const electronAcademic = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/modes/academic.ts"), "utf8");
  const mainCode = readFileSync(resolve(editorRoot, "../code/electron-main/codeMain.ts"), "utf8");
  const mainAcademic = readFileSync(resolve(editorRoot, "../code/electron-main/acaMain.ts"), "utf8");
  const sharedElectronRenderer = readFileSync(resolve(editorRoot, "../platform/native/electron-browser/rendererApi.ts"), "utf8");
  const sharedDisconnectedRenderer = readFileSync(resolve(editorRoot, "../platform/app-server/browser/rendererApi.ts"), "utf8");
  const sharedConnectedRenderer = readFileSync(resolve(editorRoot, "../platform/app-server/browser/webRendererApi.ts"), "utf8");
  const sharedElectronMain = readFileSync(resolve(editorRoot, "../code/electron-main/app.ts"), "utf8");
  assert.match(browserCode, /createViteDevDebugAdapterCapability/u);
  assert.doesNotMatch(browserAcademic, /DebugAdapter|debugAdapter/u);
  assert.match(electronCode, /createElectronDebugAdapterCapability/u);
  assert.doesNotMatch(electronAcademic, /DebugAdapter|debugAdapter/u);
  assert.match(mainCode, /debugAdapterIpcRoutes/u);
  assert.doesNotMatch(mainAcademic, /debugAdapterIpcRoutes/u);
  for (const sharedHost of [sharedElectronRenderer, sharedDisconnectedRenderer, sharedConnectedRenderer, sharedElectronMain]) assert.doesNotMatch(sharedHost, /new (?:Electron|Disconnected|ViteDev)DebugAdapterProcessService|debugAdapterIpcRoutes/u);
});

test("Editor engines delegate optional feature composition to product bundles", () => {
  const textHost = readFileSync(join(editorRoot, "browser/editorPart.ts"), "utf8");
  const textContribution = readFileSync(join(editorRoot, "contrib/codeEditorPart.contribution.ts"), "utf8");
  const findContribution = readFileSync(join(editorRoot, "contrib/find/browser/find.contribution.ts"), "utf8");
  const quickAccessContribution = readFileSync(join(editorRoot, "contrib/quickAccess/browser/quickAccessController.ts"), "utf8");
  const documentHost = readFileSync(join(editorRoot, "browser/editorWidget.ts"), "utf8");
  const documentContribution = readFileSync(join(editorRoot, "contrib/documentEditor.contribution.ts"), "utf8");
  const codePaneContribution = readFileSync(join(workbenchRoot, "contrib/codeEditor/browser/codeEditor.contribution.ts"), "utf8");
  const academicPaneContribution = readFileSync(join(workbenchRoot, "contrib/academic/browser/academicEditor.contribution.ts"), "utf8");
  const codeBundle = readFileSync(join(editorRoot, "editor.code.all.ts"), "utf8");
  const academicBundle = readFileSync(join(editorRoot, "editor.academic.all.ts"), "utf8");
  const standardBundle = readFileSync(join(editorRoot, "editor.all.ts"), "utf8");
  const editorContributionRegistry = readFileSync(join(editorRoot, "browser/editorContribution.ts"), "utf8");
  const optionalControllerPattern = /(?:AnchorSelect|BlockComment|BracketEditing|BracketMatch|BracketNavigation|CodeAction|CodeLens|ColorPicker|ContextMenu|CursorUndo|DiagnosticHover|DiagnosticNavigation|EditorState|Folding|FontZoom|Format|GotoLine|GotoSymbol|Hover|InPlaceReplace|InlayHints|InlineCompletions|InlineProgress|LineComment|LineJoin|LineOperations|LinkedEditing|Links|Message|MiddleScroll|MultiCursor|OccurrenceHighlight|OccurrenceSelection|ParameterHints|ReadOnlyMessage|Rename|SectionHeaders|SmartSelect|StickyScroll|SymbolIcons|TextDrop|ToggleTabFocusMode|Tokenization|Transpose|UnicodeHighlighter|UnusualLineTerminators|WordWrap)Controller/u;
  assert.doesNotMatch(textHost, /from\s+["'][^"']*\/contrib\/(?:find|folding|hover|format|rename|codeAction|collaboration|formatting)\//u);
  assert.match(textHost, /registerEditorPartFactory/u);
  assert.match(textContribution, /registerEditorPartFactory/u);
  assert.doesNotMatch(textContribution, /FindController/u);
  assert.doesNotMatch(textContribution, optionalControllerPattern);
  assert.match(textContribution, /EditingCommandController/u);
  assert.doesNotMatch(textContribution, /LanguageCompletionSessionController|RustSyntaxFactsService|LanguageDiagnosticDecorationBridge|TokenizationTextModelPart|TextDecorationCollection|LanguageBracketMatcher/u);
  const textInput = readFileSync(join(editorRoot, "browser/input/textInputController.ts"), "utf8");
  const codeEditorWidget = readFileSync(join(editorRoot, "browser/widget/codeEditor/codeEditorWidget.ts"), "utf8");
  assert.doesNotMatch(textInput, /from\s+["'][^"']*\/contrib\//u);
  assert.doesNotMatch(codeEditorWidget, /from\s+["'][^"']*\/contrib\//u);
  assert.doesNotMatch(editorContributionRegistry, /from\s+["'][^"']*\/contrib\//u);
  assert.match(findContribution, /registerEditorContribution/u);
  assert.match(quickAccessContribution, /registerEditorContribution/u);
  assert.match(standardBundle, /find\/browser\/find\.contribution/u);
  assert.match(standardBundle, /quickAccess\/browser\/quickAccessController/u);
  for (const contribution of ["bracketMatching", "clipboard", "codeAction", "comment", "folding", "gotoSymbol", "hover", "languageAnalysis", "multicursor", "placeholderText", "suggest", "tokenization", "unicodeHighlighter", "wordHighlighter"]) {
    assert.match(standardBundle, new RegExp(`contrib/${contribution}/browser/[^"']+\\.contribution`, "u"), contribution);
  }
  for (const contribution of ["dropOrPasteInto", "format", "quickAccess", "rename"]) assert.match(standardBundle, new RegExp(`contrib/${contribution}/browser/[^"']+Controller`, "u"), contribution);
  assert.doesNotMatch(codeBundle, /contrib\//u);
  assert.match(academicBundle, /documentEditor\.contribution/u);
  assert.doesNotMatch(codePaneContribution, /codeEditorPart\.contribution/u);
  assert.doesNotMatch(documentHost, /from\s+["'][^"']*\/contrib\/(?:formatting|collaboration)\/browser\//u);
  assert.match(documentHost, /getEditorContributions/u);
  assert.doesNotMatch(documentHost, /registerDocumentEditorContributionFactory/u);
  assert.match(documentContribution, /registerEditorContribution/u);
  assert.match(documentContribution, /FormattingContribution/u);
  assert.match(documentContribution, /CollaborationContribution/u);
  assert.doesNotMatch(academicPaneContribution, /codeEditorPart\.contribution/u);
  assert.doesNotMatch(academicPaneContribution, /documentEditor\.contribution/u);
  assert.match(standardBundle, /codeEditorPart\.contribution/u);
  assert.match(academicBundle, /documentEditor\.contribution/u);
});

test("Standard profile avoids mechanical contribution wrappers", () => {
  for (const feature of ["anchorSelect", "codelens", "colorPicker", "contextmenu", "cursorUndo", "dropOrPasteInto", "editorState", "fontZoom", "format", "inlayHints", "inlineCompletions", "inlineProgress", "inPlaceReplace", "lineSelection", "linkedEditing", "links", "message", "middleScroll", "parameterHints", "quickAccess", "readOnlyMessage", "rename", "smartSelect", "toggleTabFocusMode", "transpose", "wordWrap"]) {
    assert.deepEqual(collectFiles(join(editorRoot, "contrib", feature)).filter(file => file.endsWith(".contribution.ts")), [], feature);
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
    return statSync(file).isFile();
  } catch {
    return false;
  }
}
