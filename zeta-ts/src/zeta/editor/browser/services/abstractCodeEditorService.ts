import { Emitter } from '../../../base/common/event.js';
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { LinkedList } from '../../../base/common/linkedList.js';
import { type URI } from '../../../base/common/uri.js';
import { type IWidgetCodeEditor, type IWidgetCodeEditorOpenHandler, type IWidgetCodeEditorRegistry } from './codeEditorService.js';

/** Tracks live code editors and ordered open handlers without owning editor instances. */
export abstract class AbstractWidgetCodeEditorRegistry extends Disposable implements IWidgetCodeEditorRegistry {
	private readonly codeEditorAddEmitter = this._register(new Emitter<IWidgetCodeEditor>());
	private readonly codeEditorRemoveEmitter = this._register(new Emitter<IWidgetCodeEditor>());
	private readonly codeEditors = new Set<IWidgetCodeEditor>();
	private readonly openHandlers = new LinkedList<IWidgetCodeEditorOpenHandler>();
	private activeCodeEditor: IWidgetCodeEditor | undefined;

	public readonly onCodeEditorAdd = this.codeEditorAddEmitter.event;
	public readonly onCodeEditorRemove = this.codeEditorRemoveEmitter.event;

	public listCodeEditors(): readonly IWidgetCodeEditor[] {
		return Object.freeze([...this.codeEditors]);
	}

	public getActiveCodeEditor(): IWidgetCodeEditor | undefined {
		return this.activeCodeEditor;
	}

	public addCodeEditor(editor: IWidgetCodeEditor) {
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

	public setActiveCodeEditor(editor: IWidgetCodeEditor | undefined): void {
		if (editor && !this.codeEditors.has(editor)) {
			throw new RangeError('Active code editor must be registered');
		}
		this.activeCodeEditor = editor;
	}

	public registerCodeEditorOpenHandler(handler: IWidgetCodeEditorOpenHandler) {
		if (typeof handler !== 'function') {
			throw new TypeError('Code editor open handler must be a function');
		}
		return toDisposable(this.openHandlers.unshift(handler));
	}

	public async openCodeEditor(resource: URI): Promise<IWidgetCodeEditor | undefined> {
		for (const handler of this.openHandlers) {
			const editor = await handler(resource);
			if (editor) {
				return editor;
			}
		}
		return undefined;
	}
}
