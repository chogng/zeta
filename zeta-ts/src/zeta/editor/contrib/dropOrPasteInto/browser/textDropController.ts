import { addDisposableListener, stopEvent } from '../../../../base/browser/dom.js';
import { Disposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { createReadableClipboardData, readEditorClipboardText } from '../../../browser/controller/editContext/clipboardUtils.js';
import { registerEditorContribution } from '../../../browser/editorExtensions.js';
import { type EditorViewport } from '../../../browser/view.js';
import { type EditorSelectionController } from '../../../common/cursor/cursor.js';
import { createPasteTextCommand } from '../../../common/cursor/cursorTypeOperations.js';
import { TextSelection, TextSelectionSet } from '../../../common/core/selection.js';
import { type TextPosition } from '../../../common/core/text.js';
import { TEXT_FILE_TRANSFER_MAX_BYTES, selectTextFileTransfer } from './textFileTransfer.js';

/** Owns text and text-file drop operations for one editor. */
export class TextDropController extends Disposable {
	private asynchronousDropRequest = 0;

	constructor(private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController) {
		super();
		if (viewport.textModel !== selections.textModel) {
			this.dispose();
			throw new TypeError('Text drop dependencies must share one text model');
		}
		this._register(addDisposableListener<DragEvent>(viewport.element, 'dragover', event => this.handleDragOver(event)));
		this._register(addDisposableListener<DragEvent>(viewport.element, 'drop', event => this.handleDrop(event)));
		this._register(toDisposable(() => {
			this.asynchronousDropRequest += 1;
		}));
	}

	private handleDragOver(event: DragEvent): void {
		if (this.selections.readOnly || event.defaultPrevented) return;
		if (!containsText(event.dataTransfer) && !selectTextFileTransfer(event.dataTransfer?.files ?? [])) return;
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
	}

	private handleDrop(event: DragEvent): void {
		if (this.selections.readOnly || event.defaultPrevented) return;
		const text = readDropText(event.dataTransfer, this.viewport.element.ownerDocument);
		const target = this.viewport.getNearestTargetAtClientPoint(event);
		if (!target) return;
		if (text.length === 0) {
			this.dropTextFile(event, target.position);
			return;
		}
		stopEvent(event);
		this.viewport.element.focus({ preventScroll: true });
		this.selections.execute(createPasteTextCommand(this.viewport.textModel, TextSelectionSet.single(TextSelection.collapsedAt(target.position)), text));
		this.viewport.revealPosition(this.selections.selections.primary.active);
	}

	private dropTextFile(event: DragEvent, position: TextPosition): void {
		const file = selectTextFileTransfer(event.dataTransfer?.files ?? []);
		if (!file) return;
		const model = this.viewport.textModel;
		const expectedVersion = model.version;
		const request = ++this.asynchronousDropRequest;
		stopEvent(event);
		this.viewport.element.focus({ preventScroll: true });
		void file.text().then(text => {
			if (this.isDisposed || request !== this.asynchronousDropRequest || text.length > TEXT_FILE_TRANSFER_MAX_BYTES || model.version !== expectedVersion) return;
			this.selections.execute(createPasteTextCommand(model, TextSelectionSet.single(TextSelection.collapsedAt(position)), text));
			this.viewport.revealPosition(this.selections.selections.primary.active);
		}).catch(() => {
			// The supplied file could not be decoded as text.
		});
	}
}

registerEditorContribution({
	id: 'editor.contrib.dropOrPasteInto',
	install: context => {
		if (context.kind !== 'text') return;
		context.register(new TextDropController(context.viewport, context.selections));
	},
});

function containsText(dataTransfer: DataTransfer | null): boolean {
	if (!dataTransfer) return false;
	const types = Array.from(dataTransfer.types);
	return types.includes('text/plain') || types.includes('text/html');
}

function readDropText(dataTransfer: DataTransfer | null, ownerDocument: Document): string {
	return readEditorClipboardText(createReadableClipboardData(dataTransfer), ownerDocument);
}
