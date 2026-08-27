import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { BracketEditingController } from "./bracketEditingController.js";
import { BracketMatchController } from "./bracketMatchController.js";
import { BracketNavigationController } from "./bracketNavigationController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { DecorationPresentation, createStanzaDecorationSource } from "../../../browser/viewparts/decorations/decorationPresentation.js";
import { LanguageBracketMatcher } from "../common/bracketMatching.js";
import { LanguageLexicalContextIndex, TokenAwareLanguageLexicalContext } from "../../../common/languages/languageLexicalContext.js";
import { LanguageBracketColorizationIndex } from "../common/bracketColorization.js";
import { BracketColorizationSource } from "./bracketColorizationPresentation.js";
import { LanguageEditingAdapter } from "./languageEditingAdapter.js";

registerEditorContribution({ id: "editor.contrib.bracketMatching", configure: context => {
	const lexicalFallback = context.register(new LanguageLexicalContextIndex(context.model, context.languageId, context.configurations));
	const lexicalContext = new TokenAwareLanguageLexicalContext(lexicalFallback, context.getCapability(TextEditorCapability.tokenization), context.configurations);
	const largeFile = context.model.largeFile.tooLargeForTokenization;
	const matcher = context.register(new LanguageBracketMatcher(context.model, lexicalContext, largeFile ? { maxScanLineCount: 1_000 } : {}));
	const decorations = context.register(new TextDecorationCollection<void>(context.model));
	context.provideCapability(TextEditorCapability.bracketMatcher, matcher);
	context.provideCapability(TextEditorCapability.bracketDecorations, decorations);
	context.provideCapability(TextEditorCapability.languageLexicalContext, lexicalContext);
	context.addDecorationSource(createStanzaDecorationSource(decorations, () => DecorationPresentation.BracketMatch));
	context.setLanguageLexicalContext(lexicalContext);
	if (!largeFile && context.options.bracketPairColorization !== false) {
		const colorizations = context.register(new LanguageBracketColorizationIndex(context.model, lexicalContext));
		context.setBracketColorizationSource(new BracketColorizationSource(colorizations));
	}
		context.setLanguageEditing(context.register(new LanguageEditingAdapter(context.model, context.selections, context.languageId, context.configurations, lexicalContext, context.options.indentation)));
}, install: context => {
	if (context.kind !== "text") return;
	const matcher = context.getCapability(TextEditorCapability.bracketMatcher);
	context.register(new BracketMatchController(context.selections, matcher, context.getCapability(TextEditorCapability.bracketDecorations)));
	context.register(new BracketNavigationController(context.view.element, context.viewport, context.selections, matcher));
	context.register(new BracketEditingController(context.view.element, context.viewport, context.selections, matcher));
} });
