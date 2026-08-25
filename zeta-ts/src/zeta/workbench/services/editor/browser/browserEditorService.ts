import type { IEditorPart } from "../../../browser/parts/editor/editorPart.js";
import type { EditorInput, EditorOpenOptions, EditorOpenTarget, IEditorService } from "../common/editorService.js";
import type { EditorPartChangeEvent, EditorPartState, IEditorStateService } from "../common/editorState.js";
import type { Event } from "../../../../base/common/event.js";

/** Projects the Editor Part into the resource-oriented Workbench editor contract. */
export class BrowserEditorService implements IEditorService, IEditorStateService {
	constructor(private readonly editorPart: IEditorPart) {}

	get onDidChangeEditors(): Event<EditorPartChangeEvent> {
		return this.editorPart.onDidChangeEditors;
	}

	getEditorState(): EditorPartState {
		return this.editorPart.getEditorState();
	}

	async openEditor(input: EditorInput, options?: EditorOpenOptions, target?: EditorOpenTarget): Promise<void> {
		await this.editorPart.openEditor(input, options, target);
		if (options?.preserveFocus !== true) this.editorPart.focus();
	}

	focusActiveEditor(): void {
		this.editorPart.focus();
	}
}
