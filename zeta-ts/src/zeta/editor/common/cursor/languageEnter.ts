import { getEditorIndentationUnit, getLeadingIndentation, normalizeEditorIndentation, normalizeEditorIndentationText, resolveEditorIndentationOptions, unshiftEditorIndentation, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from "../core/misc/indentation.js";
import { TypeWithoutInterceptorsOperation, type SelectionEdit } from './cursorTypeEditOperations.js';
import { EditorCommandHistoryMode, type EditorEditCommand } from "../commands/editorEditCommand.js";
import { IndentAction, type EnterAction, type OnEnterRule } from '../languages/languageConfiguration.js';
import { type ResolvedLanguageConfiguration } from '../languages/languageConfigurationRegistry.js';
import { type LanguageLexicalContextSource } from "../languages/languageLexicalContext.js";
import { type Selection } from "../core/selection.js";
import type { SelectionSet } from "./selectionSet.js";
import { type TextModel } from "../model/textModel.js";

export interface LanguageEnterCommandOptions {
	readonly indentation?: EditorIndentationOptions;
	readonly lexicalContext?: LanguageLexicalContextSource;
}

/** Creates one language-aware Enter transaction for every current selection. */
export function createLanguageEnterCommand(model: TextModel, selections: SelectionSet, configuration: ResolvedLanguageConfiguration, options: LanguageEnterCommandOptions = {}): EditorEditCommand {
	assertConfiguration(configuration);
	assertOptions(model, configuration, options);
	const resolvedIndentation = resolveEditorIndentationOptions(options.indentation);
	const edits = selections.selections.map(selection => createEnterEdit(model, selection, configuration, resolvedIndentation, options.lexicalContext));
	return TypeWithoutInterceptorsOperation.getEdits(model, selections, edits, EditorCommandHistoryMode.BeginCoalescedTyping);
}

function createEnterEdit(model: TextModel, selection: Selection, configuration: ResolvedLanguageConfiguration, indentation: ResolvedEditorIndentationOptions, lexicalContext: LanguageLexicalContextSource | undefined): SelectionEdit {
	const startLine = model.getLineContent(selection.getStartPosition().lineNumber);
	const endLine = model.getLineContent(selection.getEndPosition().lineNumber);
	const originalBeforeText = startLine.slice(0, selection.startColumn - 1);
	const beforeText = lexicalContext?.getStructuralLineContent(selection.startLineNumber - 1, 0, selection.startColumn - 1) ?? originalBeforeText;
	const afterText = lexicalContext?.getStructuralLineContent(selection.endLineNumber - 1, selection.endColumn - 1, endLine.length) ?? endLine.slice(selection.endColumn - 1);
	const previousLineText = selection.startLineNumber > 1
		? lexicalContext?.getStructuralLineContent(selection.startLineNumber - 2) ?? model.getLineContent(selection.startLineNumber - 1)
		: "";
	const action = resolveEnterAction(configuration, previousLineText, beforeText, afterText);
	const insertion = createEnterInsertion(originalBeforeText, action, indentation);
	return {
		range: selection,
		text: insertion.text,
		anchorOffsetInText: insertion.caretOffset,
		activeOffsetInText: insertion.caretOffset,
	};
}

function resolveEnterAction(configuration: ResolvedLanguageConfiguration, previousLineText: string, beforeText: string, afterText: string): EnterAction {
	const explicit = (configuration.underlyingConfig.onEnterRules ?? []).find(rule => matchesOnEnterRule(rule, previousLineText, beforeText, afterText));
	if (explicit) return explicit.action;
	const bracketPairs = [...(configuration.underlyingConfig.brackets ?? [])].sort((left, right) => right[0].length - left[0].length);
	for (const [open, close] of bracketPairs) {
		if (!endsWithToken(beforeText, open)) continue;
		return startsWithToken(afterText, close)
			? { indentAction: IndentAction.IndentOutdent }
			: { indentAction: IndentAction.Indent };
	}
	const rules = configuration.indentationRules;
	if (rules && !testPattern(rules.unIndentedLinePattern, beforeText)) {
		if (testPattern(rules.increaseIndentPattern, beforeText) || testPattern(rules.indentNextLinePattern, beforeText)) {
			return { indentAction: IndentAction.Indent };
		}
		if (testPattern(rules.decreaseIndentPattern, afterText)) {
			return { indentAction: IndentAction.Outdent };
		}
	}
	return { indentAction: IndentAction.None };
}

function createEnterInsertion(beforeText: string, action: EnterAction, indentation: ResolvedEditorIndentationOptions): {
	readonly text: string;
	readonly caretOffset: number;
} {
	const leading = getLeadingIndentation(beforeText);
	const removeText = Math.min(action.removeText ?? 0, leading.length);
	const baseIndentation = normalizeEditorIndentation(leading.slice(0, leading.length - removeText), indentation);
	const unit = getEditorIndentationUnit(indentation);
	if (action.indentAction === IndentAction.IndentOutdent) {
		const firstLine = normalizeEditorIndentationText(baseIndentation + (action.appendText ?? unit), indentation);
		const text = "\n" + firstLine + "\n" + baseIndentation;
		return { text, caretOffset: 1 + firstLine.length };
	}
	const target = action.indentAction === IndentAction.Outdent
		? unshiftEditorIndentation(baseIndentation, indentation) + (action.appendText ?? "")
		: baseIndentation + (
			action.indentAction === IndentAction.Indent
				? unit + (action.appendText ?? "")
				: action.appendText ?? ""
		);
	const normalized = normalizeEditorIndentationText(target, indentation);
	return { text: "\n" + normalized, caretOffset: 1 + normalized.length };
}

function matchesOnEnterRule(rule: OnEnterRule, previousLineText: string, beforeText: string, afterText: string): boolean {
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
	if (typeof configuration !== 'object' || configuration === null || typeof configuration.getAutoClosingPairs !== 'function') {
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
