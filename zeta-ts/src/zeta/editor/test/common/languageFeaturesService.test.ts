import assert from 'node:assert/strict';
import test from 'node:test';
import { URI } from '../../../base/common/uri.js';
import { ComposableLanguageConfigurationService } from '../../common/languages/ownedLanguageConfigurationContributions.js';
import { LanguageFeaturesService } from '../../common/services/languageFeaturesService.js';
import { LanguageService } from '../../common/services/languageService.js';

test('language identity, configuration, and feature providers have separate owners', () => {
	using languageService = new LanguageService();
	using languageConfigurationService = new ComposableLanguageConfigurationService();
	using languageFeaturesService = new LanguageFeaturesService(languageConfigurationService);

	assert.equal(languageService.resolveLanguageId({ resource: URI.file('C:\\project\\source.ts') }), undefined);
	assert.equal(languageConfigurationService.getLanguageConfiguration('typescript').comments.lineComment, undefined);
	assert.deepEqual(languageFeaturesService.hoverProvider.getProviders('demo'), []);

	using language = languageService.registerLanguage({ id: 'demo', extensions: ['.demo'] });
	using configuration = languageConfigurationService.register('demo', { comments: { lineComment: '//' } });
	using hover = languageFeaturesService.hoverProvider.register({ languageIds: ['demo'], provideHover: () => undefined });

	assert.equal(languageService.resolveLanguageId({ resource: URI.file('C:\\project\\source.demo') }), 'demo');
	assert.equal(languageConfigurationService.getLanguageConfiguration('demo').comments.lineComment, '//');
	assert.equal(languageFeaturesService.hoverProvider.getProviders('demo').length, 1);
});

test('language feature registries report effective provider changes', () => {
	using languageConfigurationService = new ComposableLanguageConfigurationService();
	using languageFeaturesService = new LanguageFeaturesService(languageConfigurationService);
	let changes = 0;
	using listener = languageFeaturesService.colorProvider.onDidChange(() => changes += 1);
	const registration = languageFeaturesService.colorProvider.registerGroup([]);

	assert.equal(changes, 0);
	registration.replace([{
		languageIds: ['css'],
		provideDocumentColors: () => [],
		provideColorPresentations: () => [],
	}]);
	assert.equal(changes, 1);
	registration.replace([]);
	assert.equal(changes, 2);
	registration.dispose();
	assert.equal(changes, 2);
});
