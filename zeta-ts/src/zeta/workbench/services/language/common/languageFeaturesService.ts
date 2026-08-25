import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import { LanguageFeaturesService as EditorLanguageFeaturesService, type ILanguageFeaturesService as EditorLanguageFeaturesServiceContract } from "../../../../editor/common/services/languageService.js";
import { createJsonCompletionProvider, createJsonFormattingProvider, createJsonHoverProvider } from './jsonLanguageFeatures.js';

/** Workbench DI view of the editor language feature contract. */
export interface ILanguageFeaturesService extends EditorLanguageFeaturesServiceContract {}

export const ILanguageFeaturesService = createServiceIdentifier<ILanguageFeaturesService>("languageFeaturesService");

export type { SyntaxFeaturesOptions, LanguageCompletionFeaturesOptions } from "../../../../editor/common/services/languageService.js";

/** Adapts the editor language runtime and installs product-neutral JSON language features. */
export class LanguageFeaturesService extends EditorLanguageFeaturesService implements ILanguageFeaturesService {
	constructor() {
		super();
		this.own(this.registerCompletionProvider(createJsonCompletionProvider()));
		this.own(this.registerHoverProvider(createJsonHoverProvider()));
		this.own(this.registerFormattingProvider(createJsonFormattingProvider()));
	}
}
