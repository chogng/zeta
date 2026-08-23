import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { resolveEditorIndentationOptions, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from "../../../common/editorIndentation.js";
import { createDeleteLinesCommand, createDuplicateLinesCommand, createInsertLineCommand, createMoveLinesCommand, EditorLineDuplicateDirection, EditorLineInsertDirection, EditorLineMoveDirection } from "./linesOperations.js";
import { createLineIndentCommand, EditorLineIndentDirection } from "./lineIndentCommands.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface LineOperationsControllerOptions {
	readonly operatingSystem?: OperatingSystem;
	readonly indentation?: EditorIndentationOptions;
}

/** Routes VS Code-compatible physical-line operation and indentation chords locally. */
export class LineOperationsController extends DisposableOwner {
	private readonly targetOperatingSystem: OperatingSystem;
	private readonly indentation: ResolvedEditorIndentationOptions;

	constructor(
		input: HTMLTextAreaElement,
		private readonly viewport: EditorViewport,
		private readonly selections: EditorSelectionController,
		options: LineOperationsControllerOptions = {},
	) {
		super();
		try {
			this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
			this.indentation = resolveEditorIndentationOptions(options.indentation);
			if (viewport.textModel !== selections.textModel) {
				throw new TypeError("Aster line operation dependencies must share one text model");
			}
			this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
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
				this.selections.execute(createLineIndentCommand(
					this.viewport.textModel,
					this.selections.selections,
					event.shiftKey ? EditorLineIndentDirection.Outdent : EditorLineIndentDirection.Indent,
					this.indentation,
				));
				this.viewport.revealPosition(this.selections.selections.primary.active);
			}
			return;
		}
		if ((event.ctrlKey || event.metaKey) && event.shiftKey && !event.altKey && event.key.toLowerCase() === "k") {
			stopEvent(event);
			this.selections.execute(createDeleteLinesCommand(
				this.viewport.textModel,
				this.selections.selections,
			));
			this.viewport.revealPosition(this.selections.selections.primary.active);
			return;
		}
		if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key === "Enter") {
			stopEvent(event);
			this.selections.execute(createInsertLineCommand(
				this.viewport.textModel,
				this.selections.selections,
				event.shiftKey ? EditorLineInsertDirection.Before : EditorLineInsertDirection.After,
			));
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
			this.selections.execute(createMoveLinesCommand(
				this.viewport.textModel,
				this.selections.selections,
				moveDirection,
			));
			this.viewport.revealPosition(this.selections.selections.primary.active);
			return;
		}
		const duplicateDirection = resolveAsterDuplicateLineDirection(event, this.targetOperatingSystem);
		if (!duplicateDirection) return;
		stopEvent(event);
		this.selections.execute(createDuplicateLinesCommand(
			this.viewport.textModel,
			this.selections.selections,
			duplicateDirection,
		));
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}
}

/** Resolves a duplicate-line chord without colliding with the platform multi-cursor binding. */
export function resolveAsterDuplicateLineDirection(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">, targetOperatingSystem: OperatingSystem): EditorLineDuplicateDirection | undefined {
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
		throw new TypeError("Unknown Aster line operation operating system");
	}
	return resolved;
}
