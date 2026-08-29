import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { resolveEditorIndentationOptions, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from "../../../common/core/misc/indentation.js";
import { createDeleteLinesCommand, createDuplicateLinesCommand, createInsertLineCommand, createMoveLinesCommand, EditorLineDuplicateDirection, EditorLineInsertDirection, EditorLineMoveDirection } from "./linesOperations.js";
import { createLineIndentCommand, EditorLineIndentDirection } from "./lineIndentCommands.js";
import { type EditorViewport } from "../../../browser/view.js";
import { type EditorCommandExecutor } from '../../../browser/editorExtensions.js';

export const EditorLineOperationCommandId = Object.freeze({
	indent: 'editor.action.indentLines',
	outdent: 'editor.action.outdentLines',
	delete: 'editor.action.deleteLines',
	insertBefore: 'editor.action.insertLineBefore',
	insertAfter: 'editor.action.insertLineAfter',
	moveUp: 'editor.action.moveLinesUpAction',
	moveDown: 'editor.action.moveLinesDownAction',
	copyUp: 'editor.action.copyLinesUpAction',
	copyDown: 'editor.action.copyLinesDownAction',
});

export interface LineOperationsControllerOptions {
	readonly operatingSystem?: OperatingSystem;
	readonly indentation?: EditorIndentationOptions;
	readonly executeCommand?: EditorCommandExecutor;
}

/** Routes VS Code-compatible physical-line operation and indentation chords locally. */
export class LineOperationsController extends Disposable {
	private readonly targetOperatingSystem: OperatingSystem;
	private readonly indentation: ResolvedEditorIndentationOptions;
	private readonly executeCommand: EditorCommandExecutor;

	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selections: CursorsController,
		options: LineOperationsControllerOptions = {},
	) {
		super();
		try {
			this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
			this.indentation = resolveEditorIndentationOptions(options.indentation);
			this.executeCommand = options.executeCommand ?? ((_commandId, operation) => operation());
			if (viewport.textModel !== selections.textModel) {
				throw new TypeError("Stanza line operation dependencies must share one text model");
			}
			this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if (event.key === "Tab" && !event.ctrlKey && !event.altKey && !event.metaKey) {
			const hasRange = this.selections.selections.selections.some(selection => !selection.collapsed);
			if (event.shiftKey || hasRange) {
				stopEvent(event);
				const direction = event.shiftKey ? EditorLineIndentDirection.Outdent : EditorLineIndentDirection.Indent;
				const command = createLineIndentCommand(
					this.viewport.textModel,
					this.selections.selections,
					direction,
					this.indentation,
				);
				this.executeCommand(direction === EditorLineIndentDirection.Outdent ? EditorLineOperationCommandId.outdent : EditorLineOperationCommandId.indent, () => this.selections.execute(command));
				this.viewport.revealPosition(this.selections.selections.primary.active);
			}
			return;
		}
		if ((event.ctrlKey || event.metaKey) && event.shiftKey && !event.altKey && event.key.toLowerCase() === "k") {
			stopEvent(event);
			const command = createDeleteLinesCommand(
				this.viewport.textModel,
				this.selections.selections,
			);
			this.executeCommand(EditorLineOperationCommandId.delete, () => this.selections.execute(command));
			this.viewport.revealPosition(this.selections.selections.primary.active);
			return;
		}
		if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key === "Enter") {
			stopEvent(event);
			const direction = event.shiftKey ? EditorLineInsertDirection.Before : EditorLineInsertDirection.After;
			const command = createInsertLineCommand(
				this.viewport.textModel,
				this.selections.selections,
				direction,
			);
			this.executeCommand(direction === EditorLineInsertDirection.Before ? EditorLineOperationCommandId.insertBefore : EditorLineOperationCommandId.insertAfter, () => this.selections.execute(command));
			this.viewport.revealPosition(this.selections.selections.primary.active);
			return;
		}
		if (!event.altKey) return;
		if (!event.shiftKey) {
			if (event.ctrlKey || event.metaKey) return;
			const moveDirection = event.key === "ArrowUp"
				? EditorLineMoveDirection.Up
				: event.key === "ArrowDown"
					? EditorLineMoveDirection.Down
					: undefined;
			if (!moveDirection) return;
			stopEvent(event);
			const command = createMoveLinesCommand(
				this.viewport.textModel,
				this.selections.selections,
				moveDirection,
			);
			this.executeCommand(moveDirection === EditorLineMoveDirection.Up ? EditorLineOperationCommandId.moveUp : EditorLineOperationCommandId.moveDown, () => this.selections.execute(command));
			this.viewport.revealPosition(this.selections.selections.primary.active);
			return;
		}
		const duplicateDirection = resolveStanzaDuplicateLineDirection(event, this.targetOperatingSystem);
		if (!duplicateDirection) return;
		stopEvent(event);
		const command = createDuplicateLinesCommand(
			this.viewport.textModel,
			this.selections.selections,
			duplicateDirection,
		);
		this.executeCommand(duplicateDirection === EditorLineDuplicateDirection.Up ? EditorLineOperationCommandId.copyUp : EditorLineOperationCommandId.copyDown, () => this.selections.execute(command));
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}
}

/** Resolves a duplicate-line chord without colliding with the platform multi-cursor binding. */
export function resolveStanzaDuplicateLineDirection(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">, targetOperatingSystem: OperatingSystem): EditorLineDuplicateDirection | undefined {
	const direction = event.key === "ArrowUp"
		? EditorLineDuplicateDirection.Up
		: event.key === "ArrowDown"
			? EditorLineDuplicateDirection.Down
			: undefined;
	if (!direction || !event.altKey || !event.shiftKey || event.metaKey) return undefined;
	if (targetOperatingSystem === OperatingSystem.Linux) {
		return event.ctrlKey ? direction : undefined;
	}
	return event.ctrlKey ? undefined : direction;
}

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
	const resolved = value ?? operatingSystem;
	if (!Object.values(OperatingSystem).includes(resolved)) {
		throw new TypeError("Unknown Stanza line operation operating system");
	}
	return resolved;
}
