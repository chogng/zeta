import { type EditorCapability } from "../browser/editorContribution.js";
import { type LanguageDiagnostic } from "../common/languages/languageResults.js";
import { type TextDecorationCollection } from "../common/model/decorationCollection.js";
import { type LanguageBracketMatcher } from "./bracketMatching/common/bracketMatching.js";
import { type LanguageDocumentSymbolProvider } from "./documentSymbols/common/documentSymbols.js";
import { type EditorFoldingModel } from "./folding/browser/foldingModel.js";
import { type TokenizationTextModelPart } from "./tokenization/common/tokenizationTextModelPart.js";
import { type UnicodeHighlight } from "./unicodeHighlighter/common/unicodeHighlighter.js";
import { type RustSyntaxFactsService } from "../browser/services/rustSyntaxFactsService.js";
import { type SyntaxService } from "../common/languages/syntax/syntaxService.js";
import { type SemanticTokenSource } from "../browser/view/semanticTokenPresentation.js";
import { type LanguageLexicalContextSource } from "../common/languages/languageLexicalContext.js";

/** Typed identities for shared runtime objects consumed by independently selected text-editor contributions. */
export const TextEditorCapability = Object.freeze({
  bracketDecorations: capability<TextDecorationCollection<void>>("editor.capability.bracketDecorations"),
  bracketMatcher: capability<LanguageBracketMatcher>("editor.capability.bracketMatcher"),
  diagnosticDecorations: capability<TextDecorationCollection<LanguageDiagnostic>>("editor.capability.diagnosticDecorations"),
  documentSymbolProviders: capability<readonly LanguageDocumentSymbolProvider[]>("editor.capability.documentSymbolProviders"),
  folding: capability<EditorFoldingModel>("editor.capability.folding"),
  languageLexicalContext: capability<LanguageLexicalContextSource>("editor.capability.languageLexicalContext"),
  occurrenceDecorations: capability<TextDecorationCollection<void>>("editor.capability.occurrenceDecorations"),
  rustSyntaxFacts: capability<RustSyntaxFactsService | undefined>("editor.capability.rustSyntaxFacts"),
  searchDecorations: capability<TextDecorationCollection<void>>("editor.capability.searchDecorations"),
  semanticTokenSource: capability<SemanticTokenSource>("editor.capability.semanticTokenSource"),
  syntax: capability<SyntaxService>("editor.capability.syntax"),
  tokenization: capability<TokenizationTextModelPart>("editor.capability.tokenization"),
  unicodeDecorations: capability<TextDecorationCollection<UnicodeHighlight>>("editor.capability.unicodeDecorations"),
  unusualLineTerminatorDecorations: capability<TextDecorationCollection<void>>("editor.capability.unusualLineTerminatorDecorations"),
});

function capability<T>(id: string): EditorCapability<T> {
  return Object.freeze({ id });
}
