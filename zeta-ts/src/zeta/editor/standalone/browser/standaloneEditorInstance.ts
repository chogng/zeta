import type { Event } from '../../../base/common/event.js';
import { Disposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { bindColorTheme } from '../../../platform/theme/browser/themeStyles.js';
import { type IContentWidget, type IOverlayWidget, type IViewZoneChangeAccessor } from '../../browser/editorBrowser.js';
import { ConfiguredCodeEditor, type ConfiguredCodeEditorOptions, type EditorTextViewState, type IConfiguredCodeEditor } from '../../browser/configuredCodeEditor.js';
import type { EditorView } from '../../browser/editorView.js';
import type { View } from '../../browser/view.js';
import type { CodeEditorWidget } from '../../browser/widget/codeEditor/codeEditorWidget.js';
import type { CursorsController } from '../../common/cursor/cursor.js';
import type { IDimension } from '../../common/core/2d/dimension.js';
import type { Range } from '../../common/core/range.js';
import type { TextModel } from '../../common/model/textModel.js';

export interface IStandaloneEditorInstance extends IConfiguredCodeEditor {
	getModel(): TextModel;
}

/** Adapts the browser editor to standalone model and theme ownership. */
export class StandaloneEditorInstance extends Disposable implements IStandaloneEditorInstance {
	private readonly editor: ConfiguredCodeEditor;

	public readonly onDidChange: Event<void>;
	public readonly codeEditor: CodeEditorWidget;
	public readonly viewport: View;
	public readonly selections: CursorsController;
	public readonly view: EditorView;

	constructor(options: ConfiguredCodeEditorOptions, private readonly model: TextModel, ownsModel: boolean, themeService: Parameters<typeof bindColorTheme>[0]) {
		super();
		try {
			if (ownsModel) this._register(model);
			this._register(bindColorTheme(themeService, options.container));
			this.editor = this._register(new ConfiguredCodeEditor(options));
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
	public revealRange(range: Range): void { this.editor.revealRange(range); }
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
