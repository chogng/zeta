import type { Event } from '../../../base/common/event.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import { URI } from '../../../base/common/uri.js';
import { bindColorTheme } from '../../../platform/theme/browser/themeStyles.js';
import { type IContentWidget, type IOverlayWidget, type IViewZoneChangeAccessor } from '../../browser/editorBrowser.js';
import { ConfiguredCodeEditor, type ConfiguredCodeEditorOptions, type EditorTextViewState, type IConfiguredCodeEditor } from '../../browser/configuredCodeEditor.js';
import type { IWidgetCodeEditorRegistry } from '../../browser/services/codeEditorService.js';
import type { EditorView } from '../../browser/editorView.js';
import type { View } from '../../browser/view.js';
import type { CodeEditorWidget } from '../../browser/widget/codeEditor/codeEditorWidget.js';
import type { CursorsController } from '../../common/cursor/cursor.js';
import type { IDimension } from '../../common/core/2d/dimension.js';
import type { Range } from '../../common/core/range.js';
import type { ILanguageSelection, ILanguageService } from '../../common/languages/language.js';
import type { ITextModel } from '../../common/model.js';
import type { TextModel } from '../../common/model/textModel.js';
import type { IModelService } from '../../common/services/model.js';

export interface IStandaloneCodeEditor extends IConfiguredCodeEditor {
	getModel(): TextModel;
}

/** Standalone editor owner whose identity is shared by create(), editor events, and the editor registry. */
export class StandaloneEditor extends Disposable implements IStandaloneCodeEditor {
	private readonly editor: ConfiguredCodeEditor;

	public readonly onDidChange: Event<void>;
	public readonly codeEditor: CodeEditorWidget;
	public readonly viewport: View;
	public readonly selections: CursorsController;
	public readonly view: EditorView;

	constructor(options: ConfiguredCodeEditorOptions, private readonly model: TextModel, ownsModel: boolean, themeService: Parameters<typeof bindColorTheme>[0], codeEditorRegistry: IWidgetCodeEditorRegistry) {
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
			this._register(codeEditorRegistry.addCodeEditor(this));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	public getId(): string { return this.editor.getId(); }
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

/** @internal */
export function createTextModel(modelService: IModelService, languageService: ILanguageService, value: string, languageId: string | undefined, uri: URI | undefined): ITextModel {
	value ||= '';
	if (!languageId) {
		const firstLineBreak = value.indexOf('\n');
		const firstLine = firstLineBreak === -1 ? value : value.substring(0, firstLineBreak);
		return createModel(modelService, value, languageService.createByFilepathOrFirstLine(uri ?? null, firstLine), uri);
	}
	return createModel(modelService, value, languageService.createById(languageId), uri);
}

function createModel(modelService: IModelService, value: string, languageSelection: ILanguageSelection, uri: URI | undefined): ITextModel {
	return modelService.createModel(value, languageSelection, uri);
}
