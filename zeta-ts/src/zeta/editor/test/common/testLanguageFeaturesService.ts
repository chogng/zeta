import { TestLanguageConfigurationService } from './modes/testLanguageConfigurationService.js';
import { LanguageFeaturesService } from '../../common/services/languageFeaturesService.js';

/** Complete language-feature fixture with its configuration dependency. */
export class TestLanguageFeaturesService extends LanguageFeaturesService {
	public readonly languageConfigurationService: TestLanguageConfigurationService;

	constructor() {
		const languageConfigurations = new TestLanguageConfigurationService();
		super(languageConfigurations);
		this.languageConfigurationService = this._register(languageConfigurations);
	}
}
