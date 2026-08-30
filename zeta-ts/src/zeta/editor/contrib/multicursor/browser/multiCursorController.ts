import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { CursorMoveCommands } from '../../../common/cursor/cursorMoveCommands.js';
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { Position } from '../../../common/core/position.js';
import { Selection } from '../../../common/core/selection.js';
import { SelectionSet } from '../../../common/cursor/selectionSet.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type View } from "../../../browser/view.js";

export interface MultiCursorControllerOptions {
	readonly operatingSystem?: OperatingSystem;
}

/** Routes platform-specific add-cursor-above/below chords through Stanza common state. */
export class MultiCursorController extends Disposable {
	private readonly targetOperatingSystem: OperatingSystem;

	constructor(
		input: HTMLElement,
		private readonly viewport: View,
		private readonly selections: CursorsController,
		options: MultiCursorControllerOptions = {},
	) {
		super();
		try {
			this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
			if (viewport.textModel !== selections.textModel) {
				throw new TypeError("Stanza multi-cursor dependencies must share one text model");
			}
			this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if (event.shiftKey && event.altKey && !event.ctrlKey && !event.metaKey && event.key.toLowerCase() === "i") {
			const next = addCursorsToSelectedLineEnds(this.viewport.textModel, this.selections.selections);
			if (next === this.selections.selections) return;
			stopEvent(event);
			this.selections.setCursorSelections(next);
			this.viewport.revealPosition(next.primary.getPosition());
			return;
		}
		const direction = resolveStanzaAdjacentCursorDirection(event, this.targetOperatingSystem);
		if (!direction) return;
		stopEvent(event);
		const next = direction === 'up'
			? CursorMoveCommands.addCursorUp(this.viewport.textModel, this.selections.selections)
			: CursorMoveCommands.addCursorDown(this.viewport.textModel, this.selections.selections);
		this.selections.setCursorSelections(next);
		this.viewport.revealPosition(next.primary.getPosition());
	}
}

/** Resolves the non-conflicting VS Code add-cursor chord for a host platform. */
export function resolveStanzaAdjacentCursorDirection(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey'>, targetOperatingSystem: OperatingSystem): 'up' | 'down' | undefined {
	const direction = event.key === "ArrowUp"
		? 'up'
		: event.key === "ArrowDown"
			? 'down'
			: undefined;
	if (!direction) return undefined;
	if (targetOperatingSystem === OperatingSystem.Macintosh) {
		return event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey ? direction : undefined;
	}
	if (targetOperatingSystem === OperatingSystem.Windows) {
		return event.ctrlKey && event.altKey && !event.metaKey && !event.shiftKey ? direction : undefined;
	}
	return event.ctrlKey && event.shiftKey && event.altKey && !event.metaKey ? direction : undefined;
}

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
	const resolved = value ?? operatingSystem;
	if (!Object.values(OperatingSystem).includes(resolved)) {
		throw new TypeError("Unknown Stanza multi-cursor operating system");
	}
	return resolved;
}

function addCursorsToSelectedLineEnds(model: TextModel, selections: SelectionSet): SelectionSet {
	const nextSelections: Selection[] = [];
	let primaryIndex: number | undefined;
	for (let selectionIndex = 0; selectionIndex < selections.selections.length; selectionIndex += 1) {
		const selection = selections.selections[selectionIndex]!;
		if (selection.isEmpty()) continue;
		const startIndex = nextSelections.length;
		for (let lineNumber = selection.startLineNumber; lineNumber < selection.endLineNumber; lineNumber += 1) {
			appendUniqueCaret(nextSelections, new Position(lineNumber, model.getLineContent(lineNumber).length + 1));
		}
		if (selection.endColumn > 1) {
			appendUniqueCaret(nextSelections, selection.getEndPosition());
		}
		if (selectionIndex === selections.primaryIndex && nextSelections.length > startIndex) {
			primaryIndex = startIndex;
		}
	}
	if (nextSelections.length === 0) return selections;
	return SelectionSet.withPrimary(nextSelections, primaryIndex ?? 0);
}

function appendUniqueCaret(target: Selection[], position: Position): void {
	if (target.some(selection => selection.isEmpty() && Position.compare(selection.getPosition(), position) === 0)) return;
	target.push(Selection.fromPositions(position));
}
