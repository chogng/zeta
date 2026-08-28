import { getEditorIndentationUnit, getLeadingIndentation, normalizeEditorIndentation, normalizeEditorIndentationText, resolveEditorIndentationOptions, unshiftEditorIndentation, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from "../editorIndentation.js";
import { createSelectionEditCommand, type EditorSelectionEdit } from "./cursorTypeEditOperations.js";
import { EditorCommandHistoryMode, type EditorEditCommand } from "../commands/editorEditCommand.js";
import { LanguageIndentAction, type LanguageEnterAction, type LanguageOnEnterRule, type ResolvedLanguageConfiguration } from "../languages/languageConfiguration.js";
import { type LanguageLexicalContextSource } from "../languages/languageLexicalContext.js";
import { type TextSelection, type TextSelectionSet } from "../core/selection.js";
import { type TextModel } from "../model/textModel.js";

export interface LanguageEnterCommandOptions {
	readonly indentation?: EditorIndentationOptions;
	readonly lexicalContext?: LanguageLexicalContextSource;
}

/** Creates one language-aware Enter transaction for every current selection. */
export function createLanguageEnterCommand(model: TextModel, selections: TextSelectionSet, configuration: ResolvedLanguageConfiguration, options: LanguageEnterCommandOptions = {}): EditorEditCommand {
	assertConfiguration(configuration);
	assertOptions(model, configuration, options);
	const resolvedIndentation = resolveEditorIndentationOptions(options.indentation);
	const edits = selections.selections.map(selection => createEnterEdit(model, selection, configuration, resolvedIndentation, options.lexicalContext));
	return createSelectionEditCommand(model, selections, edits, EditorCommandHistoryMode.BeginCoalescedTyping);
}

function createEnterEdit(model: TextModel, selection: TextSelection, configuration: ResolvedLanguageConfiguration, indentation: ResolvedEditorIndentationOptions, lexicalContext: LanguageLexicalContextSource | undefined): EditorSelectionEdit {
	const startLine = model.getLineContent(selection.range.start.lineIndex);
	const endLine = model.getLineContent(selection.range.end.lineIndex);
	const originalBeforeText = startLine.slice(0, selection.range.start.columnIndex);
	const beforeText = lexicalContext?.getStructuralLineContent(selection.range.start.lineIndex, 0, selection.range.start.columnIndex) ?? originalBeforeText;
	const afterText = lexicalContext?.getStructuralLineContent(selection.range.end.lineIndex, selection.range.end.columnIndex, endLine.length) ?? endLine.slice(selection.range.end.columnIndex);
	const previousLineText = selection.range.start.lineIndex > 0
		? lexicalContext?.getStructuralLineContent(selection.range.start.lineIndex - 1) ?? model.getLineContent(selection.range.start.lineIndex - 1)
		: "";
	const action = resolveEnterAction(configuration, previousLineText, beforeText, afterText);
	const insertion = createEnterInsertion(originalBeforeText, action, indentation);
	return {
		range: selection.range,
		text: insertion.text,
		anchorOffsetInText: insertion.caretOffset,
		activeOffsetInText: insertion.caretOffset,
	};
}

function resolveEnterAction(configuration: ResolvedLanguageConfiguration, previousLineText: string, beforeText: string, afterText: string): LanguageEnterAction {
	const explicit = configuration.onEnterRules.find(rule => matchesOnEnterRule(rule, previousLineText, beforeText, afterText));
	if (explicit) return explicit.action;
	const bracketPairs = [...configuration.brackets].sort((left, right) => right.open.length - left.open.length);
	for (const pair of bracketPairs) {
		if (!endsWithToken(beforeText, pair.open)) continue;
		return startsWithToken(afterText, pair.close)
			? { indentAction: LanguageIndentAction.IndentOutdent }
			: { indentAction: LanguageIndentAction.Indent };
	}
	const rules = configuration.indentationRules;
	if (rules && !testPattern(rules.unIndentedLinePattern, beforeText)) {
		if (testPattern(rules.increaseIndentPattern, beforeText) || testPattern(rules.indentNextLinePattern, beforeText)) {
			return { indentAction: LanguageIndentAction.Indent };
		}
		if (testPattern(rules.decreaseIndentPattern, afterText)) {
			return { indentAction: LanguageIndentAction.Outdent };
		}
	}
	return { indentAction: LanguageIndentAction.None };
}

function createEnterInsertion(beforeText: string, action: LanguageEnterAction, indentation: ResolvedEditorIndentationOptions): {
	readonly text: string;
	readonly caretOffset: number;
} {
	const leading = getLeadingIndentation(beforeText);
	const removeText = Math.min(action.removeText ?? 0, leading.length);
	const baseIndentation = normalizeEditorIndentation(leading.slice(0, leading.length - removeText), indentation);
	const unit = getEditorIndentationUnit(indentation);
	if (action.indentAction === LanguageIndentAction.IndentOutdent) {
		const firstLine = normalizeEditorIndentationText(baseIndentation + (action.appendText ?? unit), indentation);
		const text = "\n" + firstLine + "\n" + baseIndentation;
		return { text, caretOffset: 1 + firstLine.length };
	}
	const target = action.indentAction === LanguageIndentAction.Outdent
		? unshiftEditorIndentation(baseIndentation, indentation) + (action.appendText ?? "")
		: baseIndentation + (
			action.indentAction === LanguageIndentAction.Indent
				? unit + (action.appendText ?? "")
				: action.appendText ?? ""
		);
	const normalized = normalizeEditorIndentationText(target, indentation);
	return { text: "\n" + normalized, caretOffset: 1 + normalized.length };
}

function matchesOnEnterRule(rule: LanguageOnEnterRule, previousLineText: string, beforeText: string, afterText: string): boolean {
	return testPattern(rule.beforeText, beforeText) &&
		(rule.afterText === undefined || testPattern(rule.afterText, afterText)) &&
		(rule.previousLineText === undefined || testPattern(rule.previousLineText, previousLineText));
}

function testPattern(pattern: RegExp | null | undefined, text: string): boolean {
	if (!pattern) return false;
	return new RegExp(pattern.source, pattern.flags).test(text);
}

function endsWithToken(text: string, token: string): boolean {
	return text.trimEnd().endsWith(token);
}

function startsWithToken(text: string, token: string): boolean {
	return text.trimStart().startsWith(token);
}

function assertConfiguration(configuration: ResolvedLanguageConfiguration): void {
	if (typeof configuration !== "object" || configuration === null || !Array.isArray(configuration.brackets) || !Array.isArray(configuration.onEnterRules)) {
		throw new TypeError("Language Enter requires a resolved language configuration");
	}
}

function assertOptions(model: TextModel, configuration: ResolvedLanguageConfiguration, options: LanguageEnterCommandOptions): void {
	if (typeof options !== "object" || options === null) throw new TypeError("Language Enter options must be an object");
	const lexicalContext = options.lexicalContext;
	if (!lexicalContext) return;
	if (lexicalContext.textModel !== model || !lexicalContext.supportsLanguageId(configuration.languageId) || typeof lexicalContext.getStructuralLineContent !== "function") {
		throw new TypeError("Language Enter lexical context must match its model and language");
	}
}
