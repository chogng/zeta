import type { Event } from "../../../base/common/event.js";
import type { URI } from "../../../base/common/uri.js";
import { createDecorator } from "../../../platform/instantiation/common/instantiation.js";
import type { TextModel } from "../model/textModel.js";
import type { ILanguageSelection } from '../languages/language.js';

export const IModelService = createDecorator<IModelService>("modelService");

export interface IModelService {
	readonly _serviceBrand: undefined;
	createModel(value: string, languageSelection: ILanguageSelection | null, resource?: URI, isForSimpleWidget?: boolean): TextModel;
	updateModel(model: TextModel, value: string): void;
	destroyModel(resource: URI): void;
	getModel(resource: URI): TextModel | null;
	getModels(): TextModel[];
	readonly onModelAdded: Event<TextModel>;
	readonly onModelRemoved: Event<TextModel>;
	readonly onModelLanguageChanged: Event<{ readonly model: TextModel; readonly oldLanguageId: string }>;
}
