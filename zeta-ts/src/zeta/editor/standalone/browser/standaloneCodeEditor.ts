import type { IDimension } from '../../../base/browser/geometry.js';
import type { Event } from '../../../base/common/event.js';
import { Disposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { bindColorTheme } from '../../../platform/theme/browser/themeStyles.js';
import { EditorBrowser, type EditorBrowserOptions, type EditorTextViewState, type IContentWidget, type IEditorBrowser, type IOverlayWidget, type IViewZoneChangeAccessor } from '../../browser/editorBrowser.js';
import type { EditorView, EditorViewport } from '../../browser/view.js';
import type { CodeEditorWidget } from '../../browser/widget/codeEditor/codeEditorWidget.js';
import type { EditorSelectionController } from '../../common/cursor/cursor.js';
import type { TextRange } from '../../common/core/text.js';
import type { TextModel } from '../../common/model/textModel.js';

export interface IStandaloneCodeEditor extends IEditorBrowser {
	getModel(): TextModel;
}

/** Adapts the browser editor to standalone model and theme ownership. */
export class StandaloneCodeEditor extends Disposable implements IStandaloneCodeEditor {
	private readonly editor: EditorBrowser;

	public readonly onDidChange: Event<void>;
	public readonly codeEditor: CodeEditorWidget;
	public readonly viewport: EditorViewport;
	public readonly selections: EditorSelectionController;
	public readonly view: EditorView;

	constructor(options: EditorBrowserOptions, private readonly model: TextModel, ownsModel: boolean, themeService: Parameters<typeof bindColorTheme>[0]) {
		super();
		try {
			if (ownsModel) this._register(model);
			this._register(bindColorTheme(themeService, options.container));
			this.editor = this._register(new EditorBrowser(options));
			this.onDidChange = this.editor.onDidChange;
			this.codeEditor = this.editor.codeEditor;
			this.viewport = this.editor.viewport;
			this.selections = this.editor.selections;
			this.view = this.editor.view;
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	public registerEditorLifetime(registration: IDisposable): void {
		this._register(registration);
	}

	public getModel(): TextModel { return this.model; }
	public announceAccessibilityStatus(message: string): void { this.editor.announceAccessibilityStatus(message); }
	public layout(dimension: IDimension): void { this.editor.layout(dimension); }
	public focus(): void { this.editor.focus(); }
	public getValue(): string { return this.editor.getValue(); }
	public setValue(value: string): void { this.editor.setValue(value); }
	public revealRange(range: TextRange): void { this.editor.revealRange(range); }
	public getViewState(): EditorTextViewState { return this.editor.getViewState(); }
	public restoreViewState(state: EditorTextViewState): void { this.editor.restoreViewState(state); }
	public addContentWidget(widget: IContentWidget): void { this.editor.addContentWidget(widget); }
	public layoutContentWidget(widget: IContentWidget): void { this.editor.layoutContentWidget(widget); }
	public removeContentWidget(widget: IContentWidget): void { this.editor.removeContentWidget(widget); }
	public addOverlayWidget(widget: IOverlayWidget): void { this.editor.addOverlayWidget(widget); }
	public layoutOverlayWidget(widget: IOverlayWidget): void { this.editor.layoutOverlayWidget(widget); }
	public removeOverlayWidget(widget: IOverlayWidget): void { this.editor.removeOverlayWidget(widget); }
	public changeViewZones(callback: (accessor: IViewZoneChangeAccessor) => void): void { this.editor.changeViewZones(callback); }
	public prepareSave(): Promise<void> { return this.editor.prepareSave(); }
}
