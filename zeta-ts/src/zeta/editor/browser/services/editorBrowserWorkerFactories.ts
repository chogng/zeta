import { BrowserWorkerClientPort } from '../../../platform/webWorker/browser/browserWorkerClientPort.js';
import { LanguageCompletionCatalogWorkerClient } from '../../common/languages/completion/languageCompletionCatalogWire.js';
import { type LanguageCompletionWorkerFactory } from '../../common/languages/completion/languageCompletionService.js';
import { LanguageWorkerWireClient } from '../../common/languages/languageWorkerWire.js';
import { type SyntaxWorkerFactory } from '../../common/languages/syntax/syntaxService.js';
import { SyntaxModuleWorkerClient } from '../../common/languages/syntax/syntaxModuleWorkerClient.js';
import { VersionedEditorWorkerClient, type VersionedEditorWorkerFactory } from './versionedEditorWorkerClient.js';
import { editorWorkerWireCodec } from '../../common/services/editorWorkerWire.js';

/** Owns the browser Worker factories used by one Editor host. */
export class EditorBrowserWorkerFactories {
	readonly editorWorkerFactory: VersionedEditorWorkerFactory = model => new VersionedEditorWorkerClient(
		model,
		() => new LanguageWorkerWireClient(
			new BrowserWorkerClientPort(new Worker(new URL('../../common/services/editorWebWorkerMain.ts', import.meta.url), { type: 'module', name: 'zeta-editor' })),
			editorWorkerWireCodec,
		),
	);

	readonly syntaxWorkerFactory: SyntaxWorkerFactory = () => new SyntaxModuleWorkerClient(
		new BrowserWorkerClientPort(new Worker(new URL('../../common/services/syntaxWorkerMain.ts', import.meta.url), { type: 'module', name: 'zeta-syntax' })),
		{ requiredProviderModules: ['language.lexical'] },
	);

	readonly completionWorkerFactory: LanguageCompletionWorkerFactory = () => new LanguageCompletionCatalogWorkerClient(
		new BrowserWorkerClientPort(new Worker(new URL('../../common/services/languageCompletionWorkerMain.ts', import.meta.url), { type: 'module', name: 'stanza-completion' })),
		{ requiredProviderModules: ['language.word'] },
	);
}
