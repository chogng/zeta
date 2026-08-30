import { URI } from "../../../src/zeta/base/common/uri.js";
import { DisposableStore, toDisposable } from "../../../src/zeta/base/common/lifecycle.js";
import { createBrowserEditorPart } from "../../../src/zeta/workbench/contrib/codeEditor/browser/browserEditorPart.js";
import { CodeEditorPane } from "../../../src/zeta/workbench/contrib/codeEditor/browser/codeEditorPane.js";
import { LanguageConfigurationService } from "../../../src/zeta/editor/common/languages/languageConfigurationRegistry.js";
import { LanguageFeaturesService } from "../../../src/zeta/editor/common/services/languageFeaturesService.js";
import { LanguageService } from "../../../src/zeta/editor/common/services/languageService.js";
import { Position } from "../../../src/zeta/editor/common/core/position.js";
import { Selection } from "../../../src/zeta/editor/common/core/selection.js";
import { SelectionSet } from "../../../src/zeta/editor/common/cursor/selectionSet.js";
import { BrowserTextResourceStore } from "../../../src/zeta/workbench/contrib/codeEditor/browser/browserTextResourceStore.js";
import { AppServerSyntaxProviders } from "../../../src/zeta/workbench/services/language/browser/appServerSyntaxProviders.js";
import { WorkbenchLanguageFeatures } from "../../../src/zeta/workbench/services/language/browser/workbenchLanguageFeatures.js";
import { BrowserTextModelService } from "../../../src/zeta/workbench/services/textmodelResolver/browser/browserTextModelService.js";
import { TextModel } from "../../../src/zeta/editor/editor.api.js";
import "../../../src/zeta/editor/editor.code.all.js";
import { MemoryTextFiles } from "./memoryTextFiles.js";

interface IntegrationHarness {
	readonly apiText: string;
	getValue(): string;
	setValue(value: string): void;
	save(): Promise<void>;
	getSavedText(): string;
	getSyntaxAnalysisCount(): number;
	setCursors(positions: readonly { readonly lineIndex: number; readonly columnIndex: number }[], primaryIndex?: number): void;
	revealPosition(lineIndex: number, columnIndex: number): void;
	dispose(): void;
}

declare global {
	interface Window {
		zetaTextModelIntegration: IntegrationHarness;
	}
}

const root = requiredElement("#editor-root");
const disposables = new DisposableStore();
const resource = URI.parse("inmemory://editor/main.rs");
const files = new MemoryTextFiles(resource, "fn main() {\n  answer();\n}\n");
disposables.add(toDisposable(() => files.dispose()));
const resourceStore = new BrowserTextResourceStore(files);
const models = disposables.add(new BrowserTextModelService(resourceStore));
const languageService = disposables.add(new LanguageService());
const languageConfigurationService = disposables.add(new LanguageConfigurationService());
const languageFeaturesService = disposables.add(new LanguageFeaturesService(languageConfigurationService));
disposables.add(new WorkbenchLanguageFeatures(languageService, languageConfigurationService, languageFeaturesService));
let syntaxAnalysisCount = 0;
disposables.add(new AppServerSyntaxProviders(languageFeaturesService, {
	analyze: async params => {
		syntaxAnalysisCount += 1;
		return {
			revision: params.revision,
			hasErrors: true,
			tokens: [
				{ kind: "keyword", range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 0, columnIndex: 2 } } },
				{ kind: "function", range: { start: { lineIndex: 0, columnIndex: 3 }, end: { lineIndex: 0, columnIndex: 7 } } },
			],
			foldingRanges: [{ range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 2, columnIndex: 1 } } }],
			symbols: [{
				name: "main",
				kind: "function",
				range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 2, columnIndex: 1 } },
				selectionRange: { start: { lineIndex: 0, columnIndex: 3 }, end: { lineIndex: 0, columnIndex: 7 } },
			}],
			diagnostics: [{ kind: "missing", range: { start: { lineIndex: 1, columnIndex: 2 }, end: { lineIndex: 1, columnIndex: 8 } } }],
		};
	},
	selectionRanges: async params => ({ revision: params.revision, ranges: [] }),
}));
let editorPart: ReturnType<typeof createBrowserEditorPart> | undefined;
const pane = disposables.add(new CodeEditorPane(resourceStore, {
	modelService: models,
	createPart: options => {
		editorPart = createBrowserEditorPart(options);
		return editorPart;
	},
	languageResolver: languageService,
	languageConfigurationService,
	languageFeaturesService,
	cursorSmoothCaretAnimation: "explicit",
}));
const apiModel = disposables.add(new TextModel("editor-api"));

pane.create(root);
pane.layout({ width: 900, height: 420 });
await pane.setInput({ resource, label: "main.rs" }, new AbortController().signal);

window.zetaTextModelIntegration = {
	apiText: apiModel.getText(),
	getValue: () => pane.getValue(),
	setValue: value => requiredEditorPart().setValue(value),
	save: () => pane.save(),
	getSavedText: () => files.read(resource),
	getSyntaxAnalysisCount: () => syntaxAnalysisCount,
	setCursors: (positions, primaryIndex = 0) => requiredEditorPart().selections.setCursorSelections(SelectionSet.withPrimary(
		positions.map(position => Selection.fromPositions(new Position(position.lineIndex + 1, position.columnIndex + 1))),
		primaryIndex,
	)),
	revealPosition: (lineIndex, columnIndex) => requiredEditorPart().view.revealPosition(new Position(lineIndex + 1, columnIndex + 1)),
	dispose: () => disposables.dispose(),
};

function requiredEditorPart(): ReturnType<typeof createBrowserEditorPart> {
	if (!editorPart) throw new Error("Text model integration editor is missing");
	return editorPart;
}

function requiredElement(selector: string): HTMLElement {
	const element = document.querySelector<HTMLElement>(selector);
	if (!element) throw new Error(`Missing editor integration root '${selector}'`);
	return element;
}
