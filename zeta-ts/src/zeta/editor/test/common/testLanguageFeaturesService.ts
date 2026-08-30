import { ComposableLanguageConfigurationService } from '../../common/languages/ownedLanguageConfigurationContributions.js';
import { LanguageFeaturesService } from '../../common/services/languageFeaturesService.js';

/** Complete language-feature fixture with its configuration dependency. */
export class TestLanguageFeaturesService extends LanguageFeaturesService {
	public readonly languageConfigurationService: ComposableLanguageConfigurationService;

	constructor() {
		const languageConfigurations = new ComposableLanguageConfigurationService();
		super(languageConfigurations);
		this.languageConfigurationService = this._register(languageConfigurations);
	}
}
