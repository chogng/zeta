import { strict as assert } from 'node:assert';
import test from 'node:test';
import { URI } from '../../../../../base/common/uri.js';
import { Position } from '../../../../../editor/common/core/position.js';
import { LanguageCompletionService } from '../../../../../editor/common/languages/completion/languageCompletionService.js';
import { LanguageRequestStatus } from '../../../../../editor/common/languages/languageRequestCoordinator.js';
import { SyntaxService } from '../../../../../editor/common/languages/syntax/syntaxService.js';
import { TextModel } from '../../../../../editor/common/model/textModel.js';
import { ComposableLanguageConfigurationService } from '../../../../../editor/common/languages/ownedLanguageConfigurationContributions.js';
import { LanguageFeaturesService } from '../../../../../editor/common/services/languageFeaturesService.js';
import { LanguageService } from '../../../../../editor/common/services/languageService.js';
import { LanguageHoverService } from '../../../../../editor/contrib/hover/common/hover.js';
import { WorkbenchLanguageFeatures } from '../../browser/workbenchLanguageFeatures.js';

test('Workbench installs product languages while Editor owns provider registries', async () => {
	using languageService = new LanguageService();
	using languageConfigurations = new ComposableLanguageConfigurationService();
	using languageFeatures = new LanguageFeaturesService(languageConfigurations);
	using workbenchLanguages = new WorkbenchLanguageFeatures(languageService, languageConfigurations, languageFeatures);
	using model = new TextModel('const answer = 42;');
	using syntax = new SyntaxService(model, languageFeatures.syntaxProvider);
	using completions = new LanguageCompletionService(model, languageFeatures.completionProvider);

	assert.equal(languageService.resolveLanguageId({ resource: URI.file('C:\\project\\source.ts') }), 'typescript');
	assert.equal(languageConfigurations.getLanguageConfiguration('typescript').comments.lineComment, '//');
	assert.equal((await syntax.requestAll('typescript')).tokens.status, LanguageRequestStatus.Applied);
	assert.equal(completions.textModel, model);
});

test('Language features service atomically owns a replaceable cross-kind provider batch', async () => {
	using languageConfigurations = new ComposableLanguageConfigurationService();
	using languageFeatures = new LanguageFeaturesService(languageConfigurations);
	using model = new TextModel('answer', { languageId: 'typescript' });
	using hover = new LanguageHoverService(model, languageFeatures.hoverProvider);
	const registration = languageFeatures.registerProviderBatch({ hovers: [{ selector: 'typescript', provider: { provideHover: () => ({ contents: ['first'] }) } }] });

	assert.deepEqual(await hover.provideHover('typescript', new Position((0) + 1, (1) + 1)), { contents: ['first'] });
	registration.replace({ hovers: [{ selector: 'typescript', provider: { provideHover: () => ({ contents: ['second'] }) } }] });
	assert.deepEqual(await hover.provideHover('typescript', new Position((0) + 1, (1) + 1)), { contents: ['second'] });

	registration.dispose();
	assert.equal(await hover.provideHover('typescript', new Position((0) + 1, (1) + 1)), undefined);
	assert.throws(() => registration.replace({}), /disposed/);
});
