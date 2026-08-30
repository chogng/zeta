import { Disposable } from '../../../../base/common/lifecycle.js';
import { registerBuiltinLanguageConfigurations } from '../../../../editor/common/languages/languageBuiltinConfigurations.js';
import { registerBuiltinLanguageDescriptions } from '../../../../editor/common/languages/languageBuiltinDescriptions.js';
import type { IComposableLanguageConfigurationService } from '../../../../editor/common/languages/ownedLanguageConfigurationContributions.js';
import type { ILanguageFeaturesService } from '../../../../editor/common/services/languageFeatures.js';
import type { IZetaLanguageService } from '../../../../editor/common/languages/language.js';
import { createJsonCompletionProvider, createJsonFormattingProvider, createJsonHoverProvider } from '../common/jsonLanguageFeatures.js';

/** Installs the language contributions selected by the Workbench product. */
export class WorkbenchLanguageFeatures extends Disposable {
	constructor(languageService: IZetaLanguageService, languageConfigurationService: IComposableLanguageConfigurationService, languageFeaturesService: ILanguageFeaturesService) {
		super();
		this._register(registerBuiltinLanguageDescriptions(languageService.languages));
		this._register(registerBuiltinLanguageConfigurations(languageConfigurationService.configurations));
		this._register(languageFeaturesService.completionProvider.register(createJsonCompletionProvider()));
		this._register(languageFeaturesService.hoverProvider.register(['json', 'jsonc'], createJsonHoverProvider()));
		this._register(languageFeaturesService.formattingProvider.register(['json', 'jsonc'], createJsonFormattingProvider()));
	}
}
