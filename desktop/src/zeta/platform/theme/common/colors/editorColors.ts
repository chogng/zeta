import { registerColor } from "../colorRegistry.js";
import { foreground } from "./baseColors.js";

const owner = "editor.presentation";
const color = (id: string, dark: string, light: string, description: string): string => registerColor(id, { dark, light }, { description, owner });
const alias = (id: string, value: string, description: string): string => registerColor(id, { dark: value, light: value }, { description, owner });

export const semanticTokenCommentForeground = color("editor.semanticToken.commentForeground", "#6a9955", "#008000", "Foreground for comment semantic tokens.");
export const semanticTokenKeywordForeground = color("editor.semanticToken.keywordForeground", "#c586c0", "#af00db", "Foreground for keyword semantic tokens.");
export const semanticTokenStringForeground = color("editor.semanticToken.stringForeground", "#ce9178", "#a31515", "Foreground for string semantic tokens.");
export const semanticTokenNumberForeground = color("editor.semanticToken.numberForeground", "#b5cea8", "#098658", "Foreground for number semantic tokens.");
export const semanticTokenRegexpForeground = color("editor.semanticToken.regexpForeground", "#d16969", "#811f3f", "Foreground for regular-expression semantic tokens.");
export const semanticTokenTypeForeground = color("editor.semanticToken.typeForeground", "#4ec9b0", "#267f99", "Foreground for type semantic tokens.");
export const semanticTokenFunctionForeground = color("editor.semanticToken.functionForeground", "#dcdcaa", "#795e26", "Foreground for function semantic tokens.");
export const semanticTokenVariableForeground = alias("editor.semanticToken.variableForeground", foreground, "Foreground for variable semantic tokens.");
export const semanticTokenOperatorForeground = alias("editor.semanticToken.operatorForeground", foreground, "Foreground for operator semantic tokens.");
