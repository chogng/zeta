import { UriList } from '../../../../base/common/dataTransfer.js';
import { Disposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { type IClipboardPasteEvent } from '../../../browser/controller/editContext/clipboardUtils.js';
import { type ICodeEditor } from '../../../browser/editorBrowser.js';
import { EditorOption } from '../../../common/config/editorOptions.js';
import { type Selection } from '../../../common/core/selection.js';
import { Handler, type IEditorContribution } from '../../../common/editorCommon.js';
import { TEXT_FILE_TRANSFER_MAX_BYTES, selectTextFileTransfer } from './textFileTransfer.js';

export class CopyPasteController extends Disposable implements IEditorContribution {
	public static readonly ID = 'editor.contrib.copyPasteActionController';

	public static get(editor: ICodeEditor): CopyPasteController | null {
		return editor.getContribution<CopyPasteController>(CopyPasteController.ID);
	}

	private pasteRequest = 0;
	private currentPasteOperation: Promise<void> | undefined;

	constructor(private readonly editor: ICodeEditor) {
		super();
		this._register(editor.onDidPaste(event => this.handlePaste(event)));
		this._register(toDisposable(() => { this.pasteRequest += 1; }));
	}

	public async finishedPaste(): Promise<void> {
		await this.currentPasteOperation;
	}

	private handlePaste(event: IClipboardPasteEvent): void {
		if (event.isHandled || this.editor.inComposition || this.editor.getOption(EditorOption.readOnly) || !this.editor.hasModel()) return;
		const file = selectTextFileTransfer(event.clipboardData.files);
		if (file) {
			event.setHandled();
			this.readTextFile(file);
			return;
		}
		if (event.clipboardData.getData('text/plain').length > 0) return;
		const uriList = UriList.parse(event.clipboardData.getData('text/uri-list'));
		if (uriList.length === 0) return;
		event.setHandled();
		this.editor.trigger('paste', Handler.Paste, {
			text: uriList.join('\n'),
			pasteOnNewLine: false,
			multicursorText: null,
			mode: null,
		});
	}

	private readTextFile(file: { readonly text: () => Promise<string> }): void {
		const model = this.editor.getModel();
		const selections = this.editor.getSelections();
		if (!model || !selections) return;
		const expectedVersion = model.getVersionId();
		const expectedSelections = [...selections];
		const request = ++this.pasteRequest;
		this.currentPasteOperation = file.text().then(text => {
			if (
				this.isDisposed
				|| request !== this.pasteRequest
				|| this.editor.inComposition
				|| text.length > TEXT_FILE_TRANSFER_MAX_BYTES
				|| model.getVersionId() !== expectedVersion
				|| !sameSelections(this.editor.getSelections(), expectedSelections)
			) return;
			this.editor.trigger('paste', Handler.Paste, {
				text,
				pasteOnNewLine: false,
				multicursorText: null,
				mode: null,
			});
		}).catch(() => {
			// A supplied text file that cannot be decoded leaves the model unchanged.
		});
	}
}

function sameSelections(current: readonly Selection[] | null, expected: readonly Selection[]): boolean {
	return current !== null
		&& current.length === expected.length
		&& current.every((selection, index) => selection.equalsSelection(expected[index]!));
}
