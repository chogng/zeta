import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { RustSyntaxDocumentSymbolProvider, RustSyntaxFactsService, RustSyntaxWorker } from "../../../browser/services/rustSyntaxFactsService.js";
import { LanguageTokenLineIndex } from "../../../common/tokens/languageTokenLineIndex.js";
import { LanguageDiagnosticDecorationBridge, LanguageDiagnosticPublisherBridge } from "../../gotoError/common/diagnosticDecorations.js";
import { TokenizationTextModelPart } from "../../tokenization/common/tokenizationTextModelPart.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { LanguageAnalysisController } from "./languageAnalysisController.js";

registerEditorContribution({
	id: "editor.contrib.languageAnalysis",
	configure: context => {
		const rustSyntaxFacts = context.options.syntaxApi ? context.register(new RustSyntaxFactsService(context.options.syntaxApi)) : undefined;
		const syntax = context.register(context.languageFeaturesService.createSyntaxService(context.model, {
			...(context.options.syntaxWorkerFactory ? { workerFactory: context.options.syntaxWorkerFactory } : {}),
			...(rustSyntaxFacts ? { workerDecorator: fallback => new RustSyntaxWorker(rustSyntaxFacts, fallback) } : {}),
		}));
		const tokenization = context.register(new TokenizationTextModelPart(new LanguageTokenLineIndex(syntax.tokens)));
		const languageDiagnostics = context.options.languageDiagnosticsService;
		if (languageDiagnostics) context.register(languageDiagnostics.acquire(context.options.input.resource, context.languageId, context.model));
		if (languageDiagnostics) context.register(new LanguageDiagnosticPublisherBridge(syntax.diagnostics, languageDiagnostics.createPublisher(context.options.input.resource)));
		const diagnostics = context.register(new LanguageDiagnosticDecorationBridge(syntax.diagnostics, languageDiagnostics, context.options.input.resource));
		const documentSymbolProviders = rustSyntaxFacts ? Object.freeze([new RustSyntaxDocumentSymbolProvider(rustSyntaxFacts)]) : Object.freeze([]);
		context.provideCapability(TextEditorCapability.syntax, syntax);
		context.provideCapability(TextEditorCapability.tokenization, tokenization);
		context.provideCapability(TextEditorCapability.diagnosticDecorations, diagnostics.decorations);
		context.provideCapability(TextEditorCapability.documentSymbolProviders, documentSymbolProviders);
		context.provideCapability(TextEditorCapability.rustSyntaxFacts, rustSyntaxFacts);
	},
	install: context => {
		if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization) return;
		context.register(new LanguageAnalysisController(context.getCapability(TextEditorCapability.syntax), context.languageId, context.options.onDidChangeLanguageSupport, context.onLanguageError));
	},
});
