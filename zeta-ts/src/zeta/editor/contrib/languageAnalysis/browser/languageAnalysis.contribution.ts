import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { LanguageTokenLineIndex } from "../../../common/tokens/languageTokenLineIndex.js";
import { LanguageDiagnosticDecorationBridge, LanguageDiagnosticPublisherBridge } from "../../gotoError/common/diagnosticDecorations.js";
import { TokenizationTextModelPart } from "../../tokenization/common/tokenizationTextModelPart.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { LanguageAnalysisController } from "./languageAnalysisController.js";
import { SyntaxService } from '../../../common/languages/syntax/syntaxService.js';
import { LanguageLexicalContextIndex, TokenAwareLanguageLexicalContext } from '../../../common/languages/languageLexicalContext.js';

registerEditorContribution({
	id: "editor.contrib.languageAnalysis",
	configure: context => {
		const syntax = context.register(new SyntaxService(context.model, context.languageFeaturesService.syntaxProvider, {
			...(context.options.syntaxWorkerFactory ? { workerFactory: context.options.syntaxWorkerFactory } : {}),
		}));
		const tokenization = context.register(new TokenizationTextModelPart(new LanguageTokenLineIndex(syntax.tokens)));
		const lexicalFallback = context.register(new LanguageLexicalContextIndex(context.model, context.languageId, context.configurations));
		const lexicalContext = context.register(new TokenAwareLanguageLexicalContext(lexicalFallback, tokenization, context.configurations));
		const languageDiagnostics = context.options.languageDiagnosticsService;
		if (languageDiagnostics) context.register(languageDiagnostics.acquire(context.options.input.resource, context.languageId, context.model));
		if (languageDiagnostics) context.register(new LanguageDiagnosticPublisherBridge(syntax.diagnostics, languageDiagnostics.createPublisher(context.options.input.resource)));
		const diagnostics = context.register(new LanguageDiagnosticDecorationBridge(syntax.diagnostics, languageDiagnostics, context.options.input.resource));
		context.provideCapability(TextEditorCapability.syntax, syntax);
		context.provideCapability(TextEditorCapability.tokenization, tokenization);
		context.provideCapability(TextEditorCapability.languageLexicalContext, lexicalContext);
		context.provideCapability(TextEditorCapability.diagnosticDecorations, diagnostics.decorations);
		context.setLanguageLexicalContext(lexicalContext);
	},
	install: context => {
		if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization) return;
		context.register(new LanguageAnalysisController(context.getCapability(TextEditorCapability.syntax), context.languageId, context.options.onDidChangeLanguageSupport, context.onLanguageError));
	},
});
