import { EditorCommandHistoryMode, type EditorEditCommand } from '../commands/editorEditCommand.js';
import { CursorNavigation, EditorCursorNavigationCommand, EditorCursorNavigationMode } from './cursorNavigation.js';
import { TypeWithoutInterceptorsOperation, type SelectionEdit } from './cursorTypeEditOperations.js';
import { SelectionSet } from './selectionSet.js';
import { Range } from '../core/range.js';
import { type TextModel } from '../model/textModel.js';

/** Converts Zeta SelectionSet word-deletion intents into the local transaction command format. */
export class SelectionSetWordOperations {
	public static deleteWordLeft(model: TextModel, selections: SelectionSet, wordPattern?: RegExp): EditorEditCommand {
		return createDeleteWordCommand(model, selections, EditorCursorNavigationCommand.WordLeft, EditorCommandHistoryMode.CoalesceBackspace, wordPattern);
	}

	public static deleteWordRight(model: TextModel, selections: SelectionSet, wordPattern?: RegExp): EditorEditCommand {
		return createDeleteWordCommand(model, selections, EditorCursorNavigationCommand.WordRight, EditorCommandHistoryMode.CoalesceDelete, wordPattern);
	}
}

function createDeleteWordCommand(model: TextModel, selections: SelectionSet, navigation: EditorCursorNavigationCommand, historyMode: EditorCommandHistoryMode, wordPattern: RegExp | undefined): EditorEditCommand {
	return TypeWithoutInterceptorsOperation.getEdits(
		model,
		selections,
		selections.selections.map(selection => {
			const range = selection.isEmpty()
				? CursorNavigation.navigate(model, SelectionSet.single(selection), {
					command: navigation,
					mode: EditorCursorNavigationMode.Extend,
					...(wordPattern ? { wordPattern } : {}),
				}).selections.primary
				: selection;
			return emptySelectionEdit(range);
		}),
		historyMode,
	);
}

function emptySelectionEdit(range: Range): SelectionEdit {
	return { range, text: '', anchorOffsetInText: 0, activeOffsetInText: 0 };
}
