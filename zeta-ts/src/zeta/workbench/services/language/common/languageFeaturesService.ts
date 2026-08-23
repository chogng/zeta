import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import { LanguageFeaturesService as EditorLanguageFeaturesService, type ILanguageFeaturesService as EditorLanguageFeaturesServiceContract } from "../../../../editor/common/services/languageService.js";

/** Workbench DI view of the editor language feature contract. */
export interface ILanguageFeaturesService extends EditorLanguageFeaturesServiceContract {}

export const ILanguageFeaturesService = createServiceIdentifier<ILanguageFeaturesService>("languageFeaturesService");

export type { SyntaxFeaturesOptions, LanguageCompletionFeaturesOptions } from "../../../../editor/common/services/languageService.js";

/** Adapts the editor language runtime to Workbench service registration. */
export class LanguageFeaturesService extends EditorLanguageFeaturesService implements ILanguageFeaturesService {}
