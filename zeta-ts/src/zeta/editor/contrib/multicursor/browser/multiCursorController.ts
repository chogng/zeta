import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { CursorMoveCommands } from '../../../common/cursor/cursorMoveCommands.js';
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { type View } from "../../../browser/view.js";
import { type IViewModel } from '../../../common/viewModel.js';
import { CursorChangeReason } from '../../../common/cursorEvents.js';
import { Position } from '../../../common/core/position.js';
import { Selection } from '../../../common/core/selection.js';
import { type TextModel } from '../../../common/model/textModel.js';

export interface MultiCursorControllerOptions {
	readonly operatingSystem?: OperatingSystem;
}

/** Routes platform-specific add-cursor-above/below chords through Stanza common state. */
export class MultiCursorController extends Disposable {
	private readonly targetOperatingSystem: OperatingSystem;

	constructor(
		input: HTMLElement,
		private readonly viewport: View,
		private readonly viewModel: IViewModel,
		private readonly selections: CursorsController,
		options: MultiCursorControllerOptions = {},
	) {
		super();
		try {
			this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
			if (viewport.textModel !== viewModel.model || viewport.textModel !== selections.context.model) {
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
			const current = this.selections.getSelections();
			const next = addCursorsToSelectedLineEnds(this.viewport.textModel, current);
			if (next === current) return;
			stopEvent(event);
			this.selections.setCursorSelections(next);
			this.viewport.revealPosition(next[0]!.getPosition());
			return;
		}
		const direction = resolveStanzaAdjacentCursorDirection(event, this.targetOperatingSystem);
		if (!direction) return;
		stopEvent(event);
		const next = direction === 'up'
			? CursorMoveCommands.addCursorUp(this.viewModel, this.viewModel.getCursorStates(), false)
			: CursorMoveCommands.addCursorDown(this.viewModel, this.viewModel.getCursorStates(), false);
		this.viewModel.setCursorStates('keyboard', CursorChangeReason.Explicit, next);
		this.viewport.revealPosition(this.viewModel.getPrimaryCursorState().modelState.position);
	}
}

/** Replaces non-empty selections with one caret at each selected physical line end. */
function addCursorsToSelectedLineEnds(model: TextModel, selections: readonly Selection[]): readonly Selection[] {
	const next: Selection[] = [];
	for (const selection of selections) {
		if (selection.isEmpty()) continue;
		for (let lineNumber = selection.startLineNumber; lineNumber < selection.endLineNumber; lineNumber += 1) {
			appendUniqueCaret(next, new Position(lineNumber, model.getLineMaxColumn(lineNumber)));
		}
		if (selection.endColumn > 1) appendUniqueCaret(next, selection.getEndPosition());
	}
	return next.length === 0 ? selections : Object.freeze(next);
}

function appendUniqueCaret(selections: Selection[], position: Position): void {
	if (!selections.some(selection => selection.isEmpty() && selection.getPosition().equals(position))) selections.push(Selection.fromPositions(position));
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
