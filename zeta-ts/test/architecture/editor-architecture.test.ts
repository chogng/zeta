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
		"common/config",
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

test("Flat editor layout keeps one TextModel owner and both mode bundles", () => {
	const requiredFiles = [
		"browser/editorBrowser.ts",
		"browser/editorBrowserRuntime.ts",
		"browser/editorDom.ts",
		"browser/editorExtensions.ts",
		"browser/coreCommands.ts",
		"browser/widget/richTextEditor/richTextEditorWidget.ts",
		"browser/widget/richTextEditor/richTextEditorWidget.css",
		"browser/view/viewOverlays.ts",
		"browser/view/viewLayer.ts",
		"browser/view/renderingContext.ts",
		"browser/view/domLineBreaksComputer.ts",
		"browser/view/dynamicViewOverlay.ts",
		"browser/view/viewUserInputEvents.ts",
		"browser/controller/compositionController.ts",
		"browser/view.ts",
		"browser/view/viewController.ts",
		"browser/controller/editContext/clipboardUtils.ts",
		"browser/controller/editContext/editContext.ts",
		"browser/controller/editContext/factory.ts",
		"browser/controller/editContext/screenReaderUtils.ts",
		"browser/controller/editContext/textArea/textAreaEditContext.ts",
		"browser/controller/editContext/textArea/textAreaEditContextInput.ts",
		"browser/controller/editContext/textArea/textAreaEditContextRegistry.ts",
		"browser/controller/editContext/textArea/textAreaEditContextState.ts",
		"browser/controller/editContext/textArea/textAreaAccessibilityController.ts",
		"browser/controller/editContext/native/nativeEditContext.ts",
		"browser/controller/editContext/native/nativeEditContextUtils.ts",
		"browser/controller/editContext/native/nativeEditContextRegistry.ts",
		"browser/controller/editContext/native/editContextFactory.ts",
		"browser/controller/editContext/native/nativeEditContext.css",
		"browser/controller/editContext/native/debugEditContext.ts",
		"browser/controller/editContext/native/screenReaderSupport.ts",
		"browser/controller/editContext/native/screenReaderContentSimple.ts",
		"browser/controller/editContext/native/screenReaderContentRich.ts",
		"browser/controller/editContext/native/screenReaderUtils.ts",
		"browser/services/rustDiffComputationService.ts",
		"common/core/position.ts",
		"common/config/diffEditor.ts",
		"common/config/diffEditorOptions.ts",
		"common/config/editorConfiguration.ts",
		"common/config/editorConfigurationSchema.ts",
		"common/config/editorOptions.ts",
		"common/config/editorZoom.ts",
		"common/config/fontInfo.ts",
		"common/config/fontInfoFromSettings.ts",
		"common/model/decorationCollection.ts",
		"common/model/textModel.ts",
		"common/cursor/editorSelectionController.ts",
		"common/services/languageService.ts",
		"contrib/gotoError/browser/gotoError.ts",
		"browser/view/viewPart.ts",
		"browser/viewparts/viewLines/viewLinesPart.ts",
		"browser/viewparts/viewLines/renderedLine.ts",
		"browser/viewparts/margin/marginPart.ts",
		"browser/viewparts/margin/lineGutterDecoration.ts",
		"browser/viewparts/marginDecorations/marginDecorationsPart.ts",
		"browser/viewparts/linesDecorations/linesDecorationsPart.ts",
		"browser/viewparts/blockDecorations/blockDecorationsPart.ts",
		"browser/viewparts/rulers/rulersPart.ts",
		"browser/viewparts/minimap/minimapProjection.ts",
		"browser/viewparts/minimap/minimapPresentation.ts",
		"browser/viewparts/minimap/minimapNavigationController.ts",
		"browser/viewparts/decorations/decorationsPart.ts",
		"browser/viewparts/indentGuides/indentGuidesPart.ts",
		"browser/viewparts/composition/compositionPart.ts",
		"browser/viewparts/selections/selectionsPart.ts",
		"browser/viewparts/viewCursors/viewCursorsPart.ts",
		"browser/viewparts/minimap/gpuMinimapRenderer.ts",
		"browser/config/fontMeasurements.ts",
		"browser/config/charWidthReader.ts",
		"browser/config/editorConfiguration.ts",
		"browser/config/domFontInfo.ts",
		"browser/config/elementSizeObserver.ts",
		"browser/config/tabFocus.ts",
		"browser/measurement/lineWidthIndex.ts",
		"common/viewModel/textMeasurer.ts",
		"common/viewModel/viewModelLines.ts",
		"common/viewModel/rangeGeometry.ts",
		"common/viewModel/visualRangeGeometry.ts",
		"common/viewModel/selectionGeometry.ts",
		"common/viewModel/visualSelectionGeometry.ts",
		"common/viewModel/visualCursorNavigation.ts",
		"common/viewModel/pointerHitTest.ts",
		"browser/viewparts/decorations/decorationPresentation.ts",
		"browser/viewparts/semanticTokens/semanticTokenPresentation.ts",
		"browser/viewparts/viewportOverlay/viewportOverlayPresentation.ts",
		"browser/viewparts/viewportOverlay/domTextGeometry.ts",
		"contrib/tokenization/common/tokenizationTextModelPart.ts",
		"contrib/semanticTokens/common/semanticTokens.ts",
		"common/editorResource.ts",
		"common/model/textBuffer.ts",
		"common/model/textBufferFactory.ts",
		"common/model/pieceTreeTextBuffer/rbTreeBase.ts",
		"common/model/pieceTreeTextBuffer/pieceTreeBase.ts",
		"common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.ts",
		"common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.ts",
		"common/model/lineDocument.ts",
		"common/model/textModelBlockState.ts",
		"common/model/lineDocumentProjection.ts",
		"common/services/textModelService.ts",
		"common/model/documentTransaction.ts",
		"contrib/academic/common/schema.ts",
		"editor.code.all.ts",
		"editor.academic.all.ts",
		"editor.all.ts",
		"editor.api.ts",
		"README.md",
		"text-engine.md",
		"document-engine.md",
	];
	for (const file of requiredFiles) assert.equal(statSafe(join(editorRoot, file)), true, file);

	const removedLegacyNames = [
		"browser/controller/inputController.ts",
		"browser/controller/inputCommandController.ts",
		"browser/controller/inputCompletionController.ts",
		"browser/controller/viewController.ts",
		"browser/controller/inputContracts.ts",
		"browser/input/textInputController.ts",
		"browser/input/textInputCommandController.ts",
		"browser/input/textInputCompletionController.ts",
		"browser/input/textInputContracts.ts",
		"browser/controller/textInputController.ts",
		"browser/controller/textInputCommandController.ts",
		"browser/controller/textInputCompletionController.ts",
		"browser/controller/textInputContracts.ts",
		"browser/controller/editContext/editContextController.ts",
		"browser/controller/editContext/editContextCommandController.ts",
		"browser/controller/editContext/editContextCompletionController.ts",
		"browser/controller/editContext/editContextContracts.ts",
		"browser/controller/editContext/editContextFactory.ts",
		"browser/controller/editContext/compositionController.ts",
		"browser/controller/textAreaInput.ts",
		"browser/controller/textAreaAccessibilityController.ts",
		"browser/editorSession.ts",
		"browser/browserEditorSession.ts",
		"common/model/decoration.ts",
		"contrib/gotoError/browser/gotoErrorController.ts",
		"contrib/indentation/browser/indentation.ts",
		"browser/view/renderedLine.ts",
		"browser/view/editorViewport.ts",
		"browser/viewModel/visualLineProjection.ts",
		"browser/viewModel/visibleLineProjection.ts",
		"browser/view/decorationLineIndex.ts",
		"browser/view/indentationGuides.ts",
		"browser/view/lineGutterDecoration.ts",
		"browser/view/diagnosticOverviewMarkers.ts",
		"browser/view/diffOverviewMarkers.ts",
		"browser/view/decorationPresentation.ts",
		"browser/view/domTextGeometry.ts",
		"browser/view/fontMetrics.ts",
		"browser/view/lineWidthIndex.ts",
		"browser/view/pointerHitTest.ts",
		"browser/view/rangeGeometry.ts",
		"browser/view/selectionGeometry.ts",
		"browser/view/semanticTokenPresentation.ts",
		"browser/view/viewportOverlayPresentation.ts",
		"browser/view/visibleLineProjection.ts",
		"browser/view/visualCursorNavigation.ts",
		"browser/view/visualLineProjection.ts",
		"browser/view/visualRangeGeometry.ts",
		"browser/view/visualSelectionGeometry.ts",
		"browser/view/minimapProjection.ts",
		"browser/view/minimapPresentation.ts",
		"browser/view/minimapNavigationController.ts",
		"text-engine-architecture.md",
		"text-engine-implementation-ledger.md",
		"document-engine-architecture.md",
		"browser/widget/embeddedTextEditor.ts",
		"browser/widget/codeBlockEditorWidget.ts",
		"common/model/documentModel.ts",
		"common/services/documentModelService.ts",
		"common/services/structuredTextModelService.ts",
		"common/model/textModelStructure.ts",
		"common/model/textModelStructureIndex.ts",
		"common/model/textModelBlockTree.ts",
		"common/model/textModelBlockSnapshot.ts",
		"contrib/academic/browser/academicCodeBlockEditor.ts",
	];
	for (const file of removedLegacyNames) assert.equal(statSafe(join(editorRoot, file)), false, file);
	assert.equal(existsSync(join(editorRoot, "browser/input")), false, "legacy browser input directory");
});

test("Editor browser retires only the editor-layer EditorPart", () => {
	const editorBrowser = readFileSync(join(editorRoot, "browser/editorBrowser.ts"), "utf8");
	assert.equal(statSafe(join(editorRoot, "browser/editorPart.ts")), false, "editor-layer EditorPart");
	assert.equal(statSafe(join(workbenchRoot, "browser/parts/editor/editorPart.ts")), true, "Workbench EditorPart");
	assert.match(editorBrowser, /export class EditorBrowser/u);
	assert.doesNotMatch(editorBrowser, /export class EditorPart/u);
});

test("Stanza owns its public protocol and DOM vocabulary without renaming the editor domain", () => {
	const api = readFileSync(join(editorRoot, "editor.api.ts"), "utf8");
	const codeInput = readFileSync(join(workbenchRoot, "contrib/codeEditor/browser/codeEditorInput.ts"), "utf8");
	const documentInput = readFileSync(join(workbenchRoot, "contrib/documentEditor/browser/documentEditorInput.ts"), "utf8");
	const diffInput = readFileSync(join(workbenchRoot, "contrib/codeEditor/browser/diffEditorInput.ts"), "utf8");
	const viewport = readFileSync(join(editorRoot, "browser/view.ts"), "utf8");
	const structuredSurface = [
		readFileSync(join(editorRoot, "browser/widget/richTextEditor/richTextEditorWidget.ts"), "utf8"),
		readFileSync(join(editorRoot, "browser/widget/richTextEditor/richTextEditorWidget.css"), "utf8"),
		readFileSync(join(editorRoot, "contrib/formatting/browser/formattingContribution.ts"), "utf8"),
		readFileSync(join(workbenchRoot, "contrib/documentEditor/browser/documentEditorPane.ts"), "utf8"),
	].join("\n");
	assert.match(api, /Stable DOM-free Stanza API/u);
	assert.match(codeInput, /stanza\.editor\.code/u);
	assert.match(documentInput, /stanza\.editor\.document/u);
	assert.match(diffInput, /stanza\.editor\.diff/u);
	assert.match(diffInput, /application\/vnd\.stanza\.editor-diff/u);
	assert.match(viewport, /stanza-editor/u);
	assert.match(structuredSurface, /stanza-document-/u);
	assert.match(structuredSurface, /stanza-structured-/u);
	assert.doesNotMatch(structuredSurface, /zeta-(?:document|structured)-/u);
	assert.equal(existsSync(resolve(editorRoot, "../stanza")), false, "parallel stanza directory");
});

test("Text engine PieceTree tests follow VS Code's common model layout", () => {
	assert.equal(statSafe(join(editorRoot, "test/common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.test.ts")), true);
	assert.equal(statSafe(join(editorRoot, "common/model/pieceTreeTextBuffer/rbTreeBase.ts")), true);
	assert.equal(statSafe(join(editorRoot, "common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.ts")), true);
	assert.equal(statSafe(join(editorRoot, "test/common/pieceTreeTextBuffer.test.ts")), false);
});

test("Window modes select independent Stanza feature implementations behind the shared Workbench entry", () => {
	const codeBundle = readFileSync(join(editorRoot, "editor.code.all.ts"), "utf8");
	const academicBundle = readFileSync(join(editorRoot, "editor.academic.all.ts"), "utf8");
	const standardBundle = readFileSync(join(editorRoot, "editor.all.ts"), "utf8");
	assert.match(codeBundle, /editor\.all/u);
	assert.doesNotMatch(codeBundle, /contrib\//u);
	assert.doesNotMatch(codeBundle, /contrib\/academic/u);
	assert.doesNotMatch(academicBundle, /editor\.all/u);
	assert.match(academicBundle, /contrib\/documentEditor\.contribution/u);
	assert.doesNotMatch(academicBundle, /workbench|academicEditor\.contribution/u);
	assert.match(standardBundle, /browser\/coreCommands/u);
	assert.doesNotMatch(standardBundle, /codeEditorPart\.contribution/u);
	assert.doesNotMatch(standardBundle, /editor\.(?:code|academic)\.all/u);

	const browserEntry = readFileSync(resolve(editorRoot, "../code/browser/workbench/workbench.ts"), "utf8");
	const electronEntry = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/workbench.ts"), "utf8");
	const codeContribution = readFileSync(resolve(editorRoot, "../code/browser/workbench/modes/code.contribution.ts"), "utf8");
	const academicContribution = readFileSync(resolve(editorRoot, "../code/browser/workbench/modes/academic.contribution.ts"), "utf8");
	const electronCodeMode = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/modes/code.ts"), "utf8");
	const electronAcademicMode = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/modes/academic.ts"), "utf8");
	for (const entry of [browserEntry, electronEntry]) {
		assert.match(entry, /__ZETA_WORKBENCH_MODE__/u);
		assert.match(entry, /resolveWorkbenchModeIdFromUrl/u);
		assert.match(entry, /satisfies Record<WorkbenchModeId/u);
		assert.match(entry, /modes\/code/u);
		assert.match(entry, /modes\/academic/u);
		assert.doesNotMatch(entry, /if\s*\([^)]*(?:code|academic)/u);
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

test("Code services are installed by mode-selected service registrations rather than UI contributions", () => {
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

test("Debug transport stays host-ready for mode reload but is projected by Code renderers only", () => {
	const browserCode = readFileSync(resolve(editorRoot, "../code/browser/workbench/modes/code.ts"), "utf8");
	const browserAcademic = readFileSync(resolve(editorRoot, "../code/browser/workbench/modes/academic.ts"), "utf8");
	const electronCode = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/modes/code.ts"), "utf8");
	const electronAcademic = readFileSync(resolve(editorRoot, "../code/electron-browser/workbench/modes/academic.ts"), "utf8");
	const main = readFileSync(resolve(editorRoot, "../code/electron-main/main.ts"), "utf8");
	const sharedElectronRenderer = readFileSync(resolve(editorRoot, "../platform/native/electron-browser/rendererApi.ts"), "utf8");
	const sharedDisconnectedRenderer = readFileSync(resolve(editorRoot, "../platform/app-server/browser/rendererApi.ts"), "utf8");
	const sharedConnectedRenderer = readFileSync(resolve(editorRoot, "../platform/app-server/browser/webRendererApi.ts"), "utf8");
	const sharedElectronMain = readFileSync(resolve(editorRoot, "../code/electron-main/app.ts"), "utf8");
	assert.match(browserCode, /createViteDevDebugAdapterCapability/u);
	assert.doesNotMatch(browserAcademic, /DebugAdapter|debugAdapter/u);
	assert.match(electronCode, /createElectronDebugAdapterCapability/u);
	assert.doesNotMatch(electronAcademic, /DebugAdapter|debugAdapter/u);
	assert.match(main, /debugAdapterIpcRoutes/u);
	for (const sharedHost of [sharedElectronRenderer, sharedDisconnectedRenderer, sharedConnectedRenderer, sharedElectronMain]) assert.doesNotMatch(sharedHost, /new (?:Electron|Disconnected|ViteDev)DebugAdapterProcessService|debugAdapterIpcRoutes/u);
});

test("Editor engines delegate optional feature composition to mode bundles", () => {
	const textHost = readFileSync(join(editorRoot, "browser/editorBrowser.ts"), "utf8");
	const runtimeSource = readFileSync(join(editorRoot, "browser/editorBrowserRuntime.ts"), "utf8");
	const coreCommands = readFileSync(join(editorRoot, "browser/coreCommands.ts"), "utf8");
	const findContribution = readFileSync(join(editorRoot, "contrib/find/browser/find.contribution.ts"), "utf8");
	const quickAccessContribution = readFileSync(join(editorRoot, "contrib/quickAccess/browser/quickAccessController.ts"), "utf8");
	const documentHost = readFileSync(join(editorRoot, "browser/widget/richTextEditor/richTextEditorWidget.ts"), "utf8");
	const documentContribution = readFileSync(join(editorRoot, "contrib/documentEditor.contribution.ts"), "utf8");
	const codePaneContribution = readFileSync(join(workbenchRoot, "contrib/codeEditor/browser/codeEditor.contribution.ts"), "utf8");
	const academicPaneContribution = readFileSync(join(workbenchRoot, "contrib/academic/browser/academicEditor.contribution.ts"), "utf8");
	const textModel = readFileSync(join(editorRoot, "common/model/textModel.ts"), "utf8");
	const codeBundle = readFileSync(join(editorRoot, "editor.code.all.ts"), "utf8");
	const academicBundle = readFileSync(join(editorRoot, "editor.academic.all.ts"), "utf8");
	const standardBundle = readFileSync(join(editorRoot, "editor.all.ts"), "utf8");
	const editorExtensionRegistry = readFileSync(join(editorRoot, "browser/editorExtensions.ts"), "utf8");
	const optionalControllerPattern = /(?:AnchorSelect|BlockComment|BracketEditing|BracketMatch|BracketNavigation|CodeAction|CodeLens|ColorPicker|ContextMenu|CursorUndo|DiagnosticHover|DiagnosticNavigation|EditorState|Folding|FontZoom|Format|GotoLine|GotoSymbol|Hover|InPlaceReplace|InlayHints|InlineCompletions|InlineProgress|LineComment|LineJoin|LineOperations|LinkedEditing|Links|Message|MiddleScroll|MultiCursor|OccurrenceHighlight|OccurrenceSelection|ParameterHints|ReadOnlyMessage|Rename|SectionHeaders|SmartSelect|StickyScroll|SymbolIcons|TextDrop|ToggleTabFocusMode|Tokenization|Transpose|UnicodeHighlighter|UnusualLineTerminators|WordWrap)Controller/u;
	assert.doesNotMatch(textHost, /from\s+["'][^"']*\/contrib\/(?:find|folding|hover|format|rename|codeAction|collaboration|formatting)\//u);
	assert.match(textHost, /EditorBrowserRuntime/u);
	assert.doesNotMatch(textHost, /registerEditorBrowserFactory|EditorBrowserFactory/u);
	assert.match(runtimeSource, /getEditorContributions/u);
	assert.doesNotMatch(runtimeSource, /from\s+["'][^"']*\/contrib\//u);
	assert.doesNotMatch(runtimeSource, optionalControllerPattern);
	assert.doesNotMatch(runtimeSource, /EditingCommandController/u);
	assert.match(coreCommands, /editor\.action\.selectAll/u);
	assert.match(coreCommands, /registerEditorContribution/u);
	assert.doesNotMatch(runtimeSource, /LanguageCompletionSessionController|RustSyntaxFactsService|LanguageDiagnosticDecorationBridge|TokenizationTextModelPart|TextDecorationCollection|LanguageBracketMatcher/u);
	const viewController = readFileSync(join(editorRoot, "browser/view/viewController.ts"), "utf8");
	const codeEditorWidget = readFileSync(join(editorRoot, "browser/widget/codeEditor/codeEditorWidget.ts"), "utf8");
	assert.doesNotMatch(viewController, /from\s+["'][^"']*\/contrib\//u);
	assert.doesNotMatch(codeEditorWidget, /from\s+["'][^"']*\/contrib\//u);
	assert.doesNotMatch(editorExtensionRegistry, /from\s+["'][^"']*\/contrib\//u);
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
	assert.doesNotMatch(academicPaneContribution, /contrib\/codeEditor|CodeEditorPane|EmbeddedTextEditorFactory|AcademicCodeBlockEditorFactory|CodeEditorWidget/u);
	assert.doesNotMatch(academicPaneContribution, /documentEditor\.contribution/u);
	assert.match(textModel, /static create\(/u);
	assert.match(textModel, /get lineDocument/u);
	assert.match(textModel, /getLineId/u);
	assert.doesNotMatch(textModel, /TextModelStructure|structureIndex|TextModelBlockTree/u);
	assert.match(documentHost, /case "codeBlock":[\s\S]*appendEditableText/u);
	assert.doesNotMatch(documentHost, /new TextModel|TextModel\.createStructured/u);
	assert.doesNotMatch(standardBundle, /codeEditorPart\.contribution/u);
	assert.match(academicBundle, /documentEditor\.contribution/u);
});

test("Multi-diff keeps generic projection in Editor and product integration in Workbench", () => {
	const widget = readFileSync(join(editorRoot, "browser/widget/multiDiffEditor/multiDiffEditorWidget.ts"), "utf8");
	const pane = readFileSync(join(workbenchRoot, "contrib/multiDiffEditor/browser/multiDiffEditorPane.ts"), "utf8");
	const input = readFileSync(join(workbenchRoot, "contrib/multiDiffEditor/browser/multiDiffEditorInput.ts"), "utf8");
	const contribution = readFileSync(join(workbenchRoot, "contrib/multiDiffEditor/browser/multiDiffEditor.contribution.ts"), "utf8");
	const sharedWorkbench = readFileSync(join(workbenchRoot, "browser/workbench.contribution.ts"), "utf8");
	assert.doesNotMatch(widget, /workbench/u);
	assert.match(pane, /MultiDiffEditorWidget/u);
	assert.match(input, /EditorInput/u);
	assert.match(contribution, /registerEditorPane/u);
	assert.match(contribution, /registerAction2/u);
	assert.match(sharedWorkbench, /contrib\/multiDiffEditor/u);
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
