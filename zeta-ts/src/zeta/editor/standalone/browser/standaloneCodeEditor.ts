import { URI } from '../../../base/common/uri.js';
import { bindColorTheme } from '../../../platform/theme/browser/themeStyles.js';
import type { IWidgetCodeEditorRegistry } from '../../browser/services/codeEditorService.js';
import { CodeEditorWidget, type CodeEditorWidgetOptions } from '../../browser/widget/codeEditor/codeEditorWidget.js';
import type { ILanguageSelection, ILanguageService } from '../../common/languages/language.js';
import type { ITextModel } from '../../common/model.js';
import type { TextModel } from '../../common/model/textModel.js';
import type { IModelService } from '../../common/services/model.js';

export interface IStandaloneCodeEditor extends CodeEditorWidget {
	getModel(): TextModel;
}

/** Standalone editor owner whose identity is shared by create(), editor events, and the editor registry. */
export class StandaloneEditor extends CodeEditorWidget implements IStandaloneCodeEditor {
	constructor(options: CodeEditorWidgetOptions, private readonly model: TextModel, ownsModel: boolean, themeService: Parameters<typeof bindColorTheme>[0], codeEditorRegistry: IWidgetCodeEditorRegistry) {
		super(options);
		try {
			if (ownsModel) this._register(model);
			this._register(bindColorTheme(themeService, options.container));
			this._register(codeEditorRegistry.addCodeEditor(this));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	public getModel(): TextModel { return this.model; }
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
