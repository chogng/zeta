import { registerColor } from "../colorRegistry.js";
import { errorForeground, foreground, mutedForeground, successForeground } from "./baseColors.js";

const owner = "editor.presentation";
const color = (id: string, dark: string, light: string, description: string): string => registerColor(id, { dark, light }, { description, owner });
const alias = (id: string, value: string, description: string): string => registerColor(id, { dark: value, light: value }, { description, owner });

export const tokenCommentForeground = color("editor.token.commentForeground", "#6a9955", "#008000", "Foreground for comment tokens independently of their syntax or semantic source.");
export const tokenKeywordForeground = color("editor.token.keywordForeground", "#c586c0", "#af00db", "Foreground for keyword tokens independently of their syntax or semantic source.");
export const tokenStringForeground = color("editor.token.stringForeground", "#ce9178", "#a31515", "Foreground for string tokens independently of their syntax or semantic source.");
export const tokenNumberForeground = color("editor.token.numberForeground", "#b5cea8", "#098658", "Foreground for number tokens independently of their syntax or semantic source.");
export const tokenRegexpForeground = color("editor.token.regexpForeground", "#d16969", "#811f3f", "Foreground for regular-expression tokens independently of their syntax or semantic source.");
export const tokenTypeForeground = color("editor.token.typeForeground", "#4ec9b0", "#267f99", "Foreground for type tokens independently of their syntax or semantic source.");
export const tokenFunctionForeground = color("editor.token.functionForeground", "#dcdcaa", "#795e26", "Foreground for function tokens independently of their syntax or semantic source.");
export const tokenVariableForeground = alias("editor.token.variableForeground", foreground, "Foreground for variable tokens independently of their syntax or semantic source.");
export const tokenOperatorForeground = alias("editor.token.operatorForeground", foreground, "Foreground for operator tokens independently of their syntax or semantic source.");
export const tokenAttributeForeground = alias("editor.token.attributeForeground", tokenVariableForeground, "Foreground for attribute tokens.");
export const tokenConstantForeground = alias("editor.token.constantForeground", tokenNumberForeground, "Foreground for constant tokens.");
export const tokenConstructorForeground = alias("editor.token.constructorForeground", tokenTypeForeground, "Foreground for constructor tokens.");
export const tokenEmbeddedForeground = alias("editor.token.embeddedForeground", foreground, "Foreground for embedded-language tokens.");
export const tokenLabelForeground = alias("editor.token.labelForeground", tokenKeywordForeground, "Foreground for label tokens.");
export const tokenModuleForeground = alias("editor.token.moduleForeground", tokenTypeForeground, "Foreground for module and namespace tokens.");
export const tokenPropertyForeground = alias("editor.token.propertyForeground", tokenVariableForeground, "Foreground for property tokens.");
export const tokenPunctuationForeground = alias("editor.token.punctuationForeground", tokenOperatorForeground, "Foreground for punctuation tokens.");

export const diffRemovedLineBackground = color("diffEditor.removedLineBackground", "#4b1818", "#ffebe9", "Background for removed diff lines.");
export const diffInsertedLineBackground = color("diffEditor.insertedLineBackground", "#173d24", "#dafbe1", "Background for inserted diff lines.");
export const diffRemovedTextBackground = color("diffEditor.removedTextBackground", "#7d2020", "#ffc6c2", "Background for removed inline diff ranges.");
export const diffInsertedTextBackground = color("diffEditor.insertedTextBackground", "#1f6f35", "#a6ebb7", "Background for inserted inline diff ranges.");
export const diffMissingLineBackground = color("diffEditor.missingLineBackground", "#202020", "#f8f8f9", "Background for a diff side without a corresponding source line.");
export const diffUnchangedRegionBackground = color("diffEditor.unchangedRegionBackground", "#1f2933", "#f1f6fc", "Background for collapsed unchanged diff regions.");
export const diffUnchangedRegionForeground = alias("diffEditor.unchangedRegionForeground", mutedForeground, "Foreground for collapsed unchanged diff regions.");
export const diffRemovedLineMarker = alias("diffEditor.removedLineMarker", errorForeground, "Marker foreground for removed diff lines.");
export const diffInsertedLineMarker = alias("diffEditor.insertedLineMarker", successForeground, "Marker foreground for inserted diff lines.");

const legacy = (id: string, value: string, description: string): string => registerColor(id, { dark: value, light: value }, { deprecated: "Use the corresponding editor.token.* token.", description, owner });
export const semanticTokenCommentForeground = legacy("editor.semanticToken.commentForeground", tokenCommentForeground, "Compatibility alias for comment token foreground.");
export const semanticTokenKeywordForeground = legacy("editor.semanticToken.keywordForeground", tokenKeywordForeground, "Compatibility alias for keyword token foreground.");
export const semanticTokenStringForeground = legacy("editor.semanticToken.stringForeground", tokenStringForeground, "Compatibility alias for string token foreground.");
export const semanticTokenNumberForeground = legacy("editor.semanticToken.numberForeground", tokenNumberForeground, "Compatibility alias for number token foreground.");
export const semanticTokenRegexpForeground = legacy("editor.semanticToken.regexpForeground", tokenRegexpForeground, "Compatibility alias for regular-expression token foreground.");
export const semanticTokenTypeForeground = legacy("editor.semanticToken.typeForeground", tokenTypeForeground, "Compatibility alias for type token foreground.");
export const semanticTokenFunctionForeground = legacy("editor.semanticToken.functionForeground", tokenFunctionForeground, "Compatibility alias for function token foreground.");
export const semanticTokenVariableForeground = legacy("editor.semanticToken.variableForeground", tokenVariableForeground, "Compatibility alias for variable token foreground.");
export const semanticTokenOperatorForeground = legacy("editor.semanticToken.operatorForeground", tokenOperatorForeground, "Compatibility alias for operator token foreground.");
