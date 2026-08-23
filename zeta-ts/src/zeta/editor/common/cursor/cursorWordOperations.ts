import { EditorCursorNavigationCommand, EditorCursorNavigationMode, navigateEditorCursors } from "./cursorNavigation.js";
import { EditorCommandHistoryMode, type EditorEditCommand } from "../commands/editorEditCommand.js";
import { createSelectionEditCommand, type EditorSelectionEdit } from "./cursorTypeEditOperations.js";
import { TextSelectionSet } from "../core/selection.js";
import { type TextRange } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";

/** Deletes each selection or the preceding editor word segment. */
export function createDeleteWordBackwardCommand(model: TextModel, selections: TextSelectionSet, wordPattern?: RegExp): EditorEditCommand {
	return createDeleteWordCommand(model, selections, EditorCursorNavigationCommand.WordLeft, EditorCommandHistoryMode.CoalesceBackspace, wordPattern);
}

/** Deletes each selection or the following editor word segment. */
export function createDeleteWordForwardCommand(model: TextModel, selections: TextSelectionSet, wordPattern?: RegExp): EditorEditCommand {
	return createDeleteWordCommand(model, selections, EditorCursorNavigationCommand.WordRight, EditorCommandHistoryMode.CoalesceDelete, wordPattern);
}

function createDeleteWordCommand(model: TextModel, selections: TextSelectionSet, navigation: EditorCursorNavigationCommand, historyMode: EditorCommandHistoryMode, wordPattern: RegExp | undefined): EditorEditCommand {
	return createSelectionEditCommand(
		model,
		selections,
		selections.selections.map(selection => {
			const range = selection.collapsed
				? navigateEditorCursors(model, TextSelectionSet.single(selection), {
					command: navigation,
					mode: EditorCursorNavigationMode.Extend,
					...(wordPattern ? { wordPattern } : {}),
				}).selections.primary.range
				: selection.range;
			return emptySelectionEdit(range);
		}),
		historyMode,
	);
}

function emptySelectionEdit(range: TextRange): EditorSelectionEdit {
	return {
		range,
		text: "",
		anchorOffsetInText: 0,
		activeOffsetInText: 0,
	};
}
