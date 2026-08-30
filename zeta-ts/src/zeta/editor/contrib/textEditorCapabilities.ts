import { type EditorCapability } from "../browser/editorExtensions.js";
import { type LanguageDiagnostic } from "../common/languages/languageResults.js";
import { type TextDecorationCollection } from "../common/model/decorationCollection.js";
import { type LanguageBracketPairs } from "../common/languages/languageBracketPairs.js";
import { type EditorFoldingModel } from "./folding/browser/foldingModel.js";
import { type UnicodeHighlight } from "./unicodeHighlighter/common/unicodeHighlights.js";
import { type SemanticTokenSource } from "../browser/viewParts/viewLines/viewLine.js";
import { type LanguageStructuralBracketSource } from "../common/languages/languageLexicalContext.js";

/** Typed identities for shared runtime objects consumed by independently selected text-editor contributions. */
export const TextEditorCapability = Object.freeze({
	bracketDecorations: capability<TextDecorationCollection<void>>("editor.capability.bracketDecorations"),
	bracketPairs: capability<LanguageBracketPairs>("editor.capability.bracketPairs"),
	diagnosticDecorations: capability<TextDecorationCollection<LanguageDiagnostic>>("editor.capability.diagnosticDecorations"),
	folding: capability<EditorFoldingModel>("editor.capability.folding"),
	languageLexicalContext: capability<LanguageStructuralBracketSource>("editor.capability.languageLexicalContext"),
	searchDecorations: capability<TextDecorationCollection<void>>("editor.capability.searchDecorations"),
	semanticTokenSource: capability<SemanticTokenSource>("editor.capability.semanticTokenSource"),
	unicodeDecorations: capability<TextDecorationCollection<UnicodeHighlight>>("editor.capability.unicodeDecorations"),
	unusualLineTerminatorDecorations: capability<TextDecorationCollection<void>>("editor.capability.unusualLineTerminatorDecorations"),
});

function capability<T>(id: string): EditorCapability<T> {
	return Object.freeze({ id });
}
