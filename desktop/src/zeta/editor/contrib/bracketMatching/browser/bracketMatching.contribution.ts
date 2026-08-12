import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { BracketEditingController } from "./bracketEditingController.js";
import { BracketMatchController } from "./bracketMatchController.js";
import { BracketNavigationController } from "./bracketNavigationController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { DecorationPresentation, createAsterDecorationSource } from "../../../browser/view/decorationPresentation.js";
import { LanguageBracketMatcher } from "../common/bracketMatching.js";
import { LanguageLexicalContextIndex, TokenAwareLanguageLexicalContext } from "../../../common/languages/languageLexicalContext.js";
import { LanguageBracketColorizationIndex } from "../common/bracketColorization.js";
import { BracketColorizationSource } from "./bracketColorizationPresentation.js";
import { LanguageEditingAdapter } from "./languageEditingAdapter.js";

registerEditorContribution({ id: "editor.contrib.bracketMatching", configure: context => {
  const lexicalFallback = context.own(new LanguageLexicalContextIndex(context.model, context.languageId, context.configurations));
  const lexicalContext = new TokenAwareLanguageLexicalContext(lexicalFallback, context.getCapability(TextEditorCapability.tokenization), context.configurations);
  const largeFile = context.model.largeFile.tooLargeForTokenization;
  const matcher = context.own(new LanguageBracketMatcher(context.model, lexicalContext, largeFile ? { maxScanLineCount: 1_000 } : {}));
  const decorations = context.own(new TextDecorationCollection<void>(context.model));
  context.provideCapability(TextEditorCapability.bracketMatcher, matcher);
  context.provideCapability(TextEditorCapability.bracketDecorations, decorations);
  context.provideCapability(TextEditorCapability.languageLexicalContext, lexicalContext);
  context.addDecorationSource(createAsterDecorationSource(decorations, () => DecorationPresentation.BracketMatch));
  context.setLanguageLexicalContext(lexicalContext);
  if (!largeFile) {
    const colorizations = context.own(new LanguageBracketColorizationIndex(context.model, lexicalContext));
    context.setBracketColorizationSource(new BracketColorizationSource(colorizations));
  }
  context.setTextInputLanguageEditing(context.own(new LanguageEditingAdapter(context.model, context.selections, context.languageId, context.configurations, lexicalContext, context.options.indentation)));
}, install: context => {
  if (context.kind !== "text") return;
  const matcher = context.getCapability(TextEditorCapability.bracketMatcher);
  context.own(new BracketMatchController(context.selections, matcher, context.getCapability(TextEditorCapability.bracketDecorations)));
  context.own(new BracketNavigationController(context.textInput.element, context.viewport, context.selections, matcher));
  context.own(new BracketEditingController(context.textInput.element, context.viewport, context.selections, matcher));
} });
