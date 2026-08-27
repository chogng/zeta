import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { createJoinLinesCommand } from "../common/lineJoin.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../../../browser/view.js";

export interface LineJoinControllerOptions {
	readonly operatingSystem?: OperatingSystem;
}

/** Routes the platform join-lines chord to Stanza's DOM-free command semantics. */
export class LineJoinController extends Disposable {
	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selections: EditorSelectionController,
		options: LineJoinControllerOptions = {},
	) {
		super();
		this.targetOperatingSystem = options.operatingSystem ?? operatingSystem;
		try {
			if (viewport.textModel !== selections.textModel) {
				throw new TypeError("Stanza line join dependencies must share one text model");
			}
			this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private readonly targetOperatingSystem: OperatingSystem;

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if (!isStanzaJoinLinesChord(event, this.targetOperatingSystem)) return;
		stopEvent(event);
		this.selections.execute(createJoinLinesCommand(this.viewport.textModel, this.selections.selections));
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}
}

/** Identifies Ctrl+J on Windows/Linux and Command+J on macOS. */
export function isStanzaJoinLinesChord(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">, targetOperatingSystem: OperatingSystem): boolean {
	if (event.shiftKey || event.altKey || event.key.toLowerCase() !== "j") return false;
	return targetOperatingSystem === OperatingSystem.Macintosh
		? event.metaKey && !event.ctrlKey
		: event.ctrlKey && !event.metaKey;
}
