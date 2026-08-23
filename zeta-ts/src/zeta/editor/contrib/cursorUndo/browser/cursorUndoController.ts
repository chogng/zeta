import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface CursorUndoControllerOptions {
	readonly operatingSystem?: OperatingSystem;
}

/** Routes the platform cursor-undo chord to selection-only history without changing document undo. */
export class CursorUndoController extends DisposableOwner {
	private readonly targetOperatingSystem: OperatingSystem;

	constructor(input: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, options: CursorUndoControllerOptions = {}) {
		super();
		try {
			this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
			if (viewport.textModel !== selections.textModel) throw new TypeError("Aster cursor undo dependencies must share one text model");
			this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if (!isCursorUndoChord(event, this.targetOperatingSystem)) return;
		if (!this.selections.undoCursorOperation()) return;
		stopEvent(event);
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}
}

registerEditorContribution({ id: "editor.contrib.cursorUndo", install: context => {
	if (context.kind !== "text") return;
	context.own(new CursorUndoController(context.textInput.element, context.viewport, context.selections));
} });

/** Resolves Aster's cursor-only undo shortcut without accepting unrelated modifiers. */
export function isCursorUndoChord(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">, targetOperatingSystem: OperatingSystem): boolean {
	if (event.key.toLowerCase() !== "u" || event.shiftKey || event.altKey) return false;
	return targetOperatingSystem === OperatingSystem.Macintosh
		? event.metaKey && !event.ctrlKey
		: event.ctrlKey && !event.metaKey;
}

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
	const resolved = value ?? operatingSystem;
	if (!Object.values(OperatingSystem).includes(resolved)) throw new TypeError("Unknown Aster cursor undo operating system");
	return resolved;
}
