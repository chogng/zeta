import type { Event } from "../../../base/common/event.js";
import type { URI } from "../../../base/common/uri.js";
import { createServiceIdentifier } from "../../../platform/instantiation/common/instantiation.js";
import type { TextModel } from "../model/textModel.js";

export interface ModelLanguageChangeEvent {
	readonly model: TextModel;
	readonly oldLanguageId: string;
	readonly newLanguageId: string;
}

/** Owns the resource and language identities of explicitly created text models. */
export interface IModelService {
	readonly onDidCreateModel: Event<TextModel>;
	readonly onWillDisposeModel: Event<TextModel>;
	readonly onDidChangeModelLanguage: Event<ModelLanguageChangeEvent>;
	createModel(value: string, languageId?: string, resource?: URI): TextModel;
	getModel(resource: URI): TextModel | undefined;
	getModels(): readonly TextModel[];
	getModelResource(model: TextModel): URI;
	getModelLanguage(model: TextModel): string;
	setModelLanguage(model: TextModel, languageId: string): void;
}

export const IModelService = createServiceIdentifier<IModelService>("modelService");
