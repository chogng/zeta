import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { operatingSystem, OperatingSystem } from "../../../../base/common/platform.js";
import { createTransposeCharactersCommand } from "../common/transpose.js";
import { type EditorSelectionController } from "../../../common/cursor/cursor.js";
import { type EditorViewport } from "../../../browser/view.js";
import { type EditorCommandExecutor } from '../../../browser/editorExtensions.js';

export const TransposeCommandId = 'editor.action.transpose';

export interface TransposeControllerOptions {
	readonly operatingSystem?: OperatingSystem;
}

/** Routes VS Code's macOS Ctrl+T transpose chord through Stanza's common command. */
export class TransposeController extends Disposable {
	private readonly targetOperatingSystem: OperatingSystem;

	constructor(
		input: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selections: EditorSelectionController,
		options: TransposeControllerOptions = {},
		private readonly executeCommand: EditorCommandExecutor = (_commandId, operation) => operation(),
	) {
		super();
		try {
			this.targetOperatingSystem = readOperatingSystem(options.operatingSystem);
			if (viewport.textModel !== selections.textModel) {
				throw new TypeError("Stanza transpose dependencies must share one text model");
			}
			this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
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
		this.executeCommand(TransposeCommandId, () => this.selections.execute(command));
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}
}

registerEditorContribution({
	id: 'editor.contrib.transpose',
	commands: [{ id: TransposeCommandId, canTriggerInlineEdits: true }],
	install: context => {
	if (context.kind !== "text") return;
	context.register(new TransposeController(context.view.element, context.viewport, context.selections, {}, context.executeCommand));
} });

function readOperatingSystem(value: OperatingSystem | undefined): OperatingSystem {
	const resolved = value ?? operatingSystem;
	if (!Object.values(OperatingSystem).includes(resolved)) {
		throw new TypeError("Unknown Stanza transpose operating system");
	}
	return resolved;
}
