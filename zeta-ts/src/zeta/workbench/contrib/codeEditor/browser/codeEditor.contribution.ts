import { getBrowserTextModelService } from "../../../services/textmodelResolver/browser/browserTextModelService.js";
import { registerEditorPane } from "../../../browser/parts/editor/editorRegistry.js";
import { getBrowserTextResourceStore } from "./browserTextResourceStore.js";
import { createBrowserEditorPart } from "./browserEditorPart.js";
import { CODE_EDITOR_ID, matchCodeEditor } from "./codeEditorInput.js";
import { CodeEditorPane } from "./codeEditorPane.js";
import { DIFF_EDITOR_ID, matchDiffEditor } from "./diffEditorInput.js";
import { DiffEditorPane } from "./diffEditorPane.js";
import { WrappingIndent } from "../../../../editor/common/config/editorOptions.js";
import { bindCodeLensCacheStorage } from "../../../../editor/contrib/codelens/browser/codeLensCache.js";
import { IStorageService } from "../../../../platform/storage/common/storage.js";
import { registerWorkbenchContribution, WorkbenchPhase } from "../../../common/contributions.js";
import { CodeEditorConfiguration, type WrappingIndentSetting } from "../common/editorConfiguration.js";

registerWorkbenchContribution("workbench.contrib.codeLensCachePersistence", WorkbenchPhase.BlockStartup, accessor => bindCodeLensCacheStorage(accessor.get(IStorageService)));

registerEditorPane({
	id: CODE_EDITOR_ID,
	name: "Stanza Code",
	canOpen: matchCodeEditor,
	create: options => {
		if (!options.textFileService) throw new Error("Stanza Code requires the Workbench text file service");
		const resourceStore = getBrowserTextResourceStore(options.textFileService);
		const configuration = options.configurationService;
		return new CodeEditorPane(resourceStore, {
			modelService: getBrowserTextModelService(resourceStore),
			createPart: createBrowserEditorPart,
			textMateService: options.textMateService,
			languageFeaturesService: options.languageFeaturesService,
			languageDiagnosticsService: options.languageDiagnosticsService,
			instantiationService: options.instantiationService,
			accessibilityService: options.accessibilityService,
			workingCopyService: options.workingCopyService,
			fontFamily: configuration?.getValue(CodeEditorConfiguration.fontFamily) || undefined,
			fontSize: configuration?.getValue(CodeEditorConfiguration.fontSize),
			lineHeight: configuration?.getValue(CodeEditorConfiguration.lineHeight),
			fontLigatures: configuration?.getValue(CodeEditorConfiguration.fontLigatures),
			experimentalGpuAcceleration: configuration?.getValue(CodeEditorConfiguration.experimentalGpuAcceleration),
			lineWrapping: configuration?.getValue(CodeEditorConfiguration.wordWrap),
			wrappingIndent: toWrappingIndent(configuration?.getValue(CodeEditorConfiguration.wrappingIndent)),
			minimap: { enabled: configuration?.getValue(CodeEditorConfiguration.minimapEnabled) !== false },
			activeLineHighlight: configuration?.getValue(CodeEditorConfiguration.highlightActiveLine) === false ? "off" : "on",
			showLineNumbers: configuration?.getValue(CodeEditorConfiguration.lineNumbers),
			showIndentationGuides: configuration?.getValue(CodeEditorConfiguration.indentationGuides),
			bracketPairColorization: configuration?.getValue(CodeEditorConfiguration.bracketPairColorization),
			matchBrackets: configuration?.getValue(CodeEditorConfiguration.matchBrackets),
			stickyScroll: configuration?.getValue(CodeEditorConfiguration.stickyScroll),
			indentation: {
				kind: configuration?.getValue(CodeEditorConfiguration.indentationKind),
				tabSize: configuration?.getValue(CodeEditorConfiguration.tabSize),
			},
			showUnicodeHighlights: configuration?.getValue(CodeEditorConfiguration.unicodeHighlights),
			suggestions: configuration?.getValue(CodeEditorConfiguration.suggestions),
			inlineCompletions: configuration?.getValue(CodeEditorConfiguration.inlineCompletions),
			parameterHints: configuration?.getValue(CodeEditorConfiguration.parameterHints),
			inlayHints: configuration?.getValue(CodeEditorConfiguration.inlayHints),
			codeLens: configuration?.getValue(CodeEditorConfiguration.codeLens),
			colorDecorators: configuration?.getValue(CodeEditorConfiguration.colorDecorators),
			colorDecoratorsActivatedOn: configuration?.getValue(CodeEditorConfiguration.colorDecoratorsActivatedOn),
			colorDecoratorsLimit: configuration?.getValue(CodeEditorConfiguration.colorDecoratorsLimit),
			defaultColorDecorators: configuration?.getValue(CodeEditorConfiguration.defaultColorDecorators),
			formatOnSave: configuration?.getValue(CodeEditorConfiguration.formatOnSave),
			find: configuration ? {
				seedSearchStringFromSelection: configuration.getValue(CodeEditorConfiguration.findSeedFromSelection),
				autoFindInSelection: configuration.getValue(CodeEditorConfiguration.findAutoFindInSelection),
				loop: configuration.getValue(CodeEditorConfiguration.findLoop),
				matchCase: configuration.getValue(CodeEditorConfiguration.findMatchCase),
				wholeWord: configuration.getValue(CodeEditorConfiguration.findWholeWord),
				regularExpression: configuration.getValue(CodeEditorConfiguration.findRegularExpression),
			} : undefined,
			insertFinalNewLine: configuration?.getValue(CodeEditorConfiguration.insertFinalNewLine),
			onSave: options.onSave,
			onOpenLocation: options.onOpenLocation,
			onApplyWorkspaceEdit: options.onApplyWorkspaceEdit,
			createDecorationSources: options.createDecorationSources,
		});
	},
});

function toWrappingIndent(value: WrappingIndentSetting | undefined): WrappingIndent | undefined {
	switch (value) {
		case "none": return WrappingIndent.None;
		case "same": return WrappingIndent.Same;
		case "indent": return WrappingIndent.Indent;
		case "deepIndent": return WrappingIndent.DeepIndent;
		default: return undefined;
	}
}

registerEditorPane({
	id: DIFF_EDITOR_ID,
	name: "Stanza Diff",
	canOpen: matchDiffEditor,
	create: options => {
		if (!options.textFileService) throw new Error("Stanza Diff requires the Workbench text file service");
		const diffService = options.diffService;
		if (!diffService) throw new Error("Stanza Diff requires the Workbench diff service");
		const resourceStore = getBrowserTextResourceStore(options.textFileService);
		const configuration = options.configurationService;
		return new DiffEditorPane(resourceStore, {
			modelService: getBrowserTextModelService(resourceStore),
			createComputationService: () => diffService.createComputationService(),
			lineHeight: configuration?.getValue(CodeEditorConfiguration.lineHeight),
			fontFamily: configuration?.getValue(CodeEditorConfiguration.fontFamily) || undefined,
			fontSize: configuration?.getValue(CodeEditorConfiguration.fontSize),
			fontLigatures: configuration?.getValue(CodeEditorConfiguration.fontLigatures),
			showLineNumbers: configuration?.getValue(CodeEditorConfiguration.diffShowLineNumbers),
			showInlineChanges: configuration?.getValue(CodeEditorConfiguration.diffShowInlineChanges),
			loopChanges: configuration?.getValue(CodeEditorConfiguration.diffLoopChanges),
			breadcrumbs: configuration?.getValue(CodeEditorConfiguration.diffBreadcrumbs),
		});
	},
});
