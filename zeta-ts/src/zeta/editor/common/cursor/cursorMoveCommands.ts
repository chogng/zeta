import { Selection } from "../core/selection.js";
import { SelectionSet } from "./selectionSet.js";
import { Position } from "../core/position.js";
import { type TextModel } from "../model/textModel.js";

/**
 * Adds a caret adjacent to every existing selection's active position.
 *
 * The existing selections remain unchanged. New carets clamp to the target
 * line length and never duplicate or overlap a retained selection.
 */
export class CursorMoveCommands {
	public static addCursorDown(model: TextModel, selections: SelectionSet): SelectionSet {
		return addAdjacentLineCursors(model, selections, 'below');
	}

	public static addCursorUp(model: TextModel, selections: SelectionSet): SelectionSet {
		return addAdjacentLineCursors(model, selections, 'above');
	}
}

function addAdjacentLineCursors(model: TextModel, selections: SelectionSet, direction: 'above' | 'below'): SelectionSet {
	const nextSelections = [...selections.selections];
	let primaryIndex = selections.primaryIndex;
	for (const selection of selections.selections) {
		const target = adjacentLinePosition(model, selection.getPosition(), direction);
		if (!target || nextSelections.some(candidate => positionOverlapsSelection(target, candidate))) continue;
		nextSelections.push(Selection.fromPositions(target));
		primaryIndex = nextSelections.length - 1;
	}
	return nextSelections.length === selections.selections.length
		? selections
		: SelectionSet.withPrimary(nextSelections, primaryIndex);
}

function adjacentLinePosition(model: TextModel, position: Position, direction: 'above' | 'below'): Position | undefined {
	const lineNumber = position.lineNumber + (direction === 'above' ? -1 : 1);
	if (lineNumber < 1 || lineNumber > model.lineCount) return undefined;
	return new Position(lineNumber, Math.min(position.column, model.getLineContent(lineNumber).length + 1));
}

function positionOverlapsSelection(position: Position, selection: Selection): boolean {
	if (selection.isEmpty()) return Position.compare(position, selection.getPosition()) === 0;
	return Position.compare(position, selection.getStartPosition()) >= 0 && Position.compare(position, selection.getEndPosition()) < 0;
}
