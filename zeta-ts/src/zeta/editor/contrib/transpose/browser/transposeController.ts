import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { createTransposeCharactersCommand } from "../../../common/cursor/cursorTranspose.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface TransposeControllerOptions {
	readonly operatingSystem?: OperatingSystem;
}

/** Routes VS Code's macOS Ctrl+T transpose chord through Aster's common command. */
export class TransposeController extends DisposableOwner {
	private readonly targetOperatingSystem: OperatingSystem;

	constructor(
		input: HTMLTextAreaElement,
		private readonly viewport: EditorViewport,
		private readonly selections: EditorSelectionController,
		options: TransposeControllerOptions = {},
	) {
		super();
		try {
			this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
			if (viewport.textModel !== selections.textModel) {
				throw new TypeError("Aster transpose dependencies must share one text model");
			}
			this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if (this.targetOperatingSystem !== OperatingSystem.Macintosh || !event.ctrlKey || event.metaKey || event.shiftKey || event.altKey || event.key.toLowerCase() !== "t") return;
		const command = createTransposeCharactersCommand(this.viewport.textModel, this.selections.selections);
		if (!command) return;
		stopEvent(event);
		this.selections.execute(command);
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}
}

registerEditorContribution({ id: "editor.contrib.transpose", install: context => {
	if (context.kind !== "text") return;
	context.own(new TransposeController(context.textInput.element, context.viewport, context.selections));
} });

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
	const resolved = value ?? operatingSystem;
	if (!Object.values(OperatingSystem).includes(resolved)) {
		throw new TypeError("Unknown Aster transpose operating system");
	}
	return resolved;
}
