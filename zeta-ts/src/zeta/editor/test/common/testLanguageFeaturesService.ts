import { LanguageConfigurationService } from '../../common/services/languageConfigurationService.js';
import { LanguageFeaturesService } from '../../common/services/languageFeaturesService.js';

/** Complete language-feature fixture with its configuration dependency. */
export class TestLanguageFeaturesService extends LanguageFeaturesService {
	public readonly languageConfigurationService: LanguageConfigurationService;

	constructor() {
		const languageConfigurations = new LanguageConfigurationService();
		super(languageConfigurations);
		this.languageConfigurationService = this._register(languageConfigurations);
	}
}
