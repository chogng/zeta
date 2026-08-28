import { BrowserWorkerClientPort } from '../../../platform/webWorker/browser/browserWorkerClientPort.js';
import { LanguageCompletionCatalogWorkerClient } from '../../common/languages/completion/languageCompletionCatalogWire.js';
import { type LanguageCompletionWorkerFactory } from '../../common/languages/completion/languageCompletionService.js';
import { type SyntaxWorkerFactory } from '../../common/languages/syntax/syntaxService.js';
import { SyntaxModuleWorkerClient } from '../../common/languages/syntax/syntaxModuleWorkerClient.js';

/** Owns the browser Worker factories used by one Editor host. */
export class EditorWorkerService {
	readonly syntaxWorkerFactory: SyntaxWorkerFactory = () => new SyntaxModuleWorkerClient(
		new BrowserWorkerClientPort(new Worker(new URL('../../common/services/syntaxWorkerMain.ts', import.meta.url), { type: 'module', name: 'zeta-syntax' })),
		{ requiredProviderModules: ['language.lexical'] },
	);

	readonly completionWorkerFactory: LanguageCompletionWorkerFactory = () => new LanguageCompletionCatalogWorkerClient(
		new BrowserWorkerClientPort(new Worker(new URL('../../common/services/languageCompletionWorkerMain.ts', import.meta.url), { type: 'module', name: 'stanza-completion' })),
		{ requiredProviderModules: ['language.word'] },
	);
}
