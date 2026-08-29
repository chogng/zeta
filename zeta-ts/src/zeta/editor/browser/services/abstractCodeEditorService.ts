import { Emitter } from '../../../base/common/event.js';
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { LinkedList } from '../../../base/common/linkedList.js';
import { type URI } from '../../../base/common/uri.js';
import { type CodeEditorWidget } from '../widget/codeEditor/codeEditorWidget.js';
import { type ICodeEditorOpenHandler, type ICodeEditorService } from './codeEditorService.js';

/** Tracks live code editors and ordered open handlers without owning editor instances. */
export abstract class AbstractCodeEditorService extends Disposable implements ICodeEditorService {
	private readonly codeEditorAddEmitter = this._register(new Emitter<CodeEditorWidget>());
	private readonly codeEditorRemoveEmitter = this._register(new Emitter<CodeEditorWidget>());
	private readonly codeEditors = new Set<CodeEditorWidget>();
	private readonly openHandlers = new LinkedList<ICodeEditorOpenHandler>();
	private activeCodeEditor: CodeEditorWidget | undefined;

	public readonly onCodeEditorAdd = this.codeEditorAddEmitter.event;
	public readonly onCodeEditorRemove = this.codeEditorRemoveEmitter.event;

	public listCodeEditors(): readonly CodeEditorWidget[] {
		return Object.freeze([...this.codeEditors]);
	}

	public getActiveCodeEditor(): CodeEditorWidget | undefined {
		return this.activeCodeEditor;
	}

	public addCodeEditor(editor: CodeEditorWidget) {
		if (this.codeEditors.has(editor)) {
			throw new RangeError('Code editor is already registered');
		}
		this.codeEditors.add(editor);
		this.codeEditorAddEmitter.fire(editor);
		return toDisposable(() => {
			if (!this.codeEditors.delete(editor)) {
				return;
			}
			if (this.activeCodeEditor === editor) {
				this.activeCodeEditor = undefined;
			}
			this.codeEditorRemoveEmitter.fire(editor);
		});
	}

	public setActiveCodeEditor(editor: CodeEditorWidget | undefined): void {
		if (editor && !this.codeEditors.has(editor)) {
			throw new RangeError('Active code editor must be registered');
		}
		this.activeCodeEditor = editor;
	}

	public registerCodeEditorOpenHandler(handler: ICodeEditorOpenHandler) {
		if (typeof handler !== 'function') {
			throw new TypeError('Code editor open handler must be a function');
		}
		return toDisposable(this.openHandlers.unshift(handler));
	}

	public async openCodeEditor(resource: URI): Promise<CodeEditorWidget | undefined> {
		for (const handler of this.openHandlers) {
			const editor = await handler(resource);
			if (editor) {
				return editor;
			}
		}
		return undefined;
	}
}
