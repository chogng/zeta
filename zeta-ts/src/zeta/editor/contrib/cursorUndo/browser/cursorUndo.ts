import { addDisposableListener, stopEvent } from '../../../../base/browser/dom.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { operatingSystem, OperatingSystem } from '../../../../base/common/platform.js';
import * as nls from '../../../../nls.js';
import { ServiceConstructionDescriptor } from '../../../../platform/instantiation/common/instantiation.js';
import { type ICodeEditor } from '../../../browser/editorBrowser.js';
import { EditorAction, EditorContributionInstantiation, registerEditorAction, registerTextEditorCapabilityContribution, type ServicesAccessor, type TextEditorContributionContext } from '../../../browser/editorExtensions.js';
import { type IEditorContribution } from '../../../common/editorCommon.js';

/** Owns bounded cursor-only undo and redo for one editor instance. */
export class CursorUndoRedoController extends Disposable implements IEditorContribution {
	static readonly ID = 'editor.contrib.cursorUndoRedoController';

	static get(editor: ICodeEditor): CursorUndoRedoController | null {
		return editor.getContribution<CursorUndoRedoController>(CursorUndoRedoController.ID);
	}

	constructor(private readonly context: TextEditorContributionContext) {
		super();
		this._register(addDisposableListener(context.view.element, 'keydown', event => {
			if (!isCursorUndoKey(event)) return;
			if (!this.context.selectionController.undoCursorOperation()) return;
			stopEvent(event);
			this.reveal();
		}));
	}

	cursorUndo(): void {
		if (this.context.selectionController.undoCursorOperation()) this.reveal();
	}

	cursorRedo(): void {
		if (this.context.selectionController.redoCursorOperation()) this.reveal();
	}

	private reveal(): void {
		this.context.viewport.revealPosition(this.context.selectionController.getSelections()[0]!.getPosition());
	}
}

export class CursorUndo extends EditorAction {
	constructor() {
		super({ id: 'cursorUndo', label: nls.localize2('cursor.undo', 'Cursor Undo'), precondition: undefined });
	}

	run(_accessor: ServicesAccessor, editor: ICodeEditor): void {
		CursorUndoRedoController.get(editor)?.cursorUndo();
	}
}

export class CursorRedo extends EditorAction {
	constructor() {
		super({ id: 'cursorRedo', label: nls.localize2('cursor.redo', 'Cursor Redo'), precondition: undefined });
	}

	run(_accessor: ServicesAccessor, editor: ICodeEditor): void {
		CursorUndoRedoController.get(editor)?.cursorRedo();
	}
}

function isCursorUndoKey(event: KeyboardEvent): boolean {
	if (event.defaultPrevented || event.isComposing || event.getModifierState('AltGraph')) return false;
	if (event.key.toLowerCase() !== 'u' || event.shiftKey || event.altKey) return false;
	return operatingSystem === OperatingSystem.Macintosh
		? event.metaKey && !event.ctrlKey
		: event.ctrlKey && !event.metaKey;
}

registerTextEditorCapabilityContribution({
	id: CursorUndoRedoController.ID,
	commands: [{ id: 'cursorUndo' }, { id: 'cursorRedo' }],
	runtime: {
		descriptor: new ServiceConstructionDescriptor(CursorUndoRedoController),
		instantiation: EditorContributionInstantiation.Eager,
	},
});
registerEditorAction(CursorUndo);
registerEditorAction(CursorRedo);
