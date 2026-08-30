import * as strings from '../../../base/common/strings.js';
import { ReplaceCommand } from '../commands/replaceCommand.js';
import { type EditorAutoClosingEditStrategy, type EditorAutoClosingStrategy } from '../config/editorOptions.js';
import { CursorConfiguration, EditOperationResult, EditOperationType, type ICursorSimpleModel, isQuote } from '../cursorCommon.js';
import { CursorColumns } from '../core/cursorColumns.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { Selection } from '../core/selection.js';
import type { ICommand } from '../editorCommon.js';
import type { StandardAutoClosingPairConditional } from '../languages/languageConfiguration.js';
import { MoveOperations } from './cursorMoveOperations.js';

export class DeleteOperations {
	public static deleteRight(previousEditOperationType: EditOperationType, configuration: CursorConfiguration, model: ICursorSimpleModel, selections: Selection[]): [boolean, Array<ICommand | null>] {
		const commands: Array<ICommand | null> = [];
		let shouldPushStackElementBefore = previousEditOperationType !== EditOperationType.DeletingRight;
		for (let index = 0; index < selections.length; index++) {
			const deleteSelection = this.getDeleteRightRange(selections[index]!, model, configuration);
			if (deleteSelection.isEmpty()) {
				commands[index] = null;
				continue;
			}
			if (deleteSelection.startLineNumber !== deleteSelection.endLineNumber) shouldPushStackElementBefore = true;
			commands[index] = new ReplaceCommand(deleteSelection, '');
		}
		return [shouldPushStackElementBefore, commands];
	}

	private static getDeleteRightRange(selection: Selection, model: ICursorSimpleModel, configuration: CursorConfiguration): Range {
		if (!selection.isEmpty()) return selection;
		const position = selection.getPosition();
		const rightOfPosition = MoveOperations.right(configuration, model, position);
		if (configuration.trimWhitespaceOnDelete && rightOfPosition.lineNumber !== position.lineNumber) {
			const currentLineHasContent = model.getLineFirstNonWhitespaceColumn(position.lineNumber) > 0;
			const firstNonWhitespaceColumn = model.getLineFirstNonWhitespaceColumn(rightOfPosition.lineNumber);
			if (currentLineHasContent && firstNonWhitespaceColumn > 0) return new Range(rightOfPosition.lineNumber, firstNonWhitespaceColumn, position.lineNumber, position.column);
		}
		return new Range(rightOfPosition.lineNumber, rightOfPosition.column, position.lineNumber, position.column);
	}

	public static isAutoClosingPairDelete(autoClosingDelete: EditorAutoClosingEditStrategy, autoClosingBrackets: EditorAutoClosingStrategy, autoClosingQuotes: EditorAutoClosingStrategy, autoClosingPairsOpen: Map<string, StandardAutoClosingPairConditional[]>, model: ICursorSimpleModel, selections: Selection[], autoClosedCharacters: Range[]): boolean {
		if (autoClosingBrackets === 'never' && autoClosingQuotes === 'never') return false;
		if (autoClosingDelete === 'never') return false;
		for (const selection of selections) {
			const position = selection.getPosition();
			if (!selection.isEmpty()) return false;
			const lineText = model.getLineContent(position.lineNumber);
			if (position.column < 2 || position.column >= lineText.length + 1) return false;
			const character = lineText.charAt(position.column - 2);
			const autoClosingPairCandidates = autoClosingPairsOpen.get(character);
			if (!autoClosingPairCandidates) return false;
			if (isQuote(character) ? autoClosingQuotes === 'never' : autoClosingBrackets === 'never') return false;
			const afterCharacter = lineText.charAt(position.column - 1);
			if (!autoClosingPairCandidates.some(candidate => candidate.open === character && candidate.close === afterCharacter)) return false;
			if (autoClosingDelete === 'auto' && !autoClosedCharacters.some(range => position.lineNumber === range.startLineNumber && position.column === range.startColumn)) return false;
		}
		return true;
	}

	private static _runAutoClosingPairDelete(_configuration: CursorConfiguration, _model: ICursorSimpleModel, selections: Selection[]): [boolean, ICommand[]] {
		return [true, selections.map(selection => {
			const position = selection.getPosition();
			return new ReplaceCommand(new Range(position.lineNumber, position.column - 1, position.lineNumber, position.column + 1), '');
		})];
	}

	public static deleteLeft(previousEditOperationType: EditOperationType, configuration: CursorConfiguration, model: ICursorSimpleModel, selections: Selection[], autoClosedCharacters: Range[]): [boolean, Array<ICommand | null>] {
		if (this.isAutoClosingPairDelete(configuration.autoClosingDelete, configuration.autoClosingBrackets, configuration.autoClosingQuotes, configuration.autoClosingPairs.autoClosingPairsOpenByEnd, model, selections, autoClosedCharacters)) return this._runAutoClosingPairDelete(configuration, model, selections);
		const commands: Array<ICommand | null> = [];
		let shouldPushStackElementBefore = previousEditOperationType !== EditOperationType.DeletingLeft;
		for (let index = 0; index < selections.length; index++) {
			const deleteRange = DeleteOperations.getDeleteLeftRange(selections[index]!, model, configuration);
			if (deleteRange.isEmpty()) {
				commands[index] = null;
				continue;
			}
			if (deleteRange.startLineNumber !== deleteRange.endLineNumber) shouldPushStackElementBefore = true;
			commands[index] = new ReplaceCommand(deleteRange, '');
		}
		return [shouldPushStackElementBefore, commands];
	}

	private static getDeleteLeftRange(selection: Selection, model: ICursorSimpleModel, configuration: CursorConfiguration): Range {
		if (!selection.isEmpty()) return selection;
		const position = selection.getPosition();
		if (configuration.useTabStops && position.column > 1) {
			const lineContent = model.getLineContent(position.lineNumber);
			const firstNonWhitespaceIndex = strings.firstNonWhitespaceIndex(lineContent);
			const lastIndentationColumn = firstNonWhitespaceIndex === -1 ? lineContent.length + 1 : firstNonWhitespaceIndex + 1;
			if (position.column <= lastIndentationColumn) {
				const fromVisibleColumn = configuration.visibleColumnFromColumn(model, position);
				const toVisibleColumn = CursorColumns.prevIndentTabStop(fromVisibleColumn, configuration.indentSize);
				const toColumn = configuration.columnFromVisibleColumn(model, position.lineNumber, toVisibleColumn);
				return new Range(position.lineNumber, toColumn, position.lineNumber, position.column);
			}
		}
		return Range.fromPositions(DeleteOperations.getPositionAfterDeleteLeft(position, model), position);
	}

	private static getPositionAfterDeleteLeft(position: Position, model: ICursorSimpleModel): Position {
		if (position.column > 1) {
			const index = strings.getLeftDeleteOffset(position.column - 1, model.getLineContent(position.lineNumber));
			return position.with(undefined, index + 1);
		}
		if (position.lineNumber > 1) {
			const lineNumber = position.lineNumber - 1;
			return new Position(lineNumber, model.getLineMaxColumn(lineNumber));
		}
		return position;
	}

	public static cut(configuration: CursorConfiguration, model: ICursorSimpleModel, selections: Selection[]): EditOperationResult {
		const commands: Array<ICommand | null> = [];
		let lastCutRange: Range | null = null;
		selections.sort((left, right) => Position.compare(left.getStartPosition(), right.getEndPosition()));
		for (let index = 0; index < selections.length; index++) {
			const selection = selections[index]!;
			if (!selection.isEmpty()) {
				commands[index] = new ReplaceCommand(selection, '');
				continue;
			}
			if (!configuration.emptySelectionClipboard) {
				commands[index] = null;
				continue;
			}
			const position = selection.getPosition();
			let deleteSelection: Range;
			if (position.lineNumber < model.getLineCount()) {
				deleteSelection = new Range(position.lineNumber, 1, position.lineNumber + 1, 1);
			} else if (position.lineNumber > 1 && lastCutRange?.endLineNumber !== position.lineNumber) {
				deleteSelection = new Range(position.lineNumber - 1, model.getLineMaxColumn(position.lineNumber - 1), position.lineNumber, model.getLineMaxColumn(position.lineNumber));
			} else {
				deleteSelection = new Range(position.lineNumber, 1, position.lineNumber, model.getLineMaxColumn(position.lineNumber));
			}
			lastCutRange = deleteSelection;
			commands[index] = deleteSelection.isEmpty() ? null : new ReplaceCommand(deleteSelection, '');
		}
		return new EditOperationResult(EditOperationType.Other, commands, {
			shouldPushStackElementBefore: true,
			shouldPushStackElementAfter: true,
		});
	}
}
