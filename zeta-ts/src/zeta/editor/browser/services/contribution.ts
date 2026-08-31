import { AbstractCodeEditorService } from './abstractCodeEditorService.js';
import { registerTextEditorCapabilityContribution } from '../editorExtensions.js';
import { MarkerDecorationsContribution } from './markerDecorations.js';
import { BrowserWorkerClientPort } from '../../../platform/webWorker/browser/browserWorkerClientPort.js';
import { LanguageCompletionCatalogWorkerClient } from '../../common/languages/completion/languageCompletionCatalogWire.js';
import { type LanguageCompletionWorkerFactory } from '../../common/languages/completion/languageCompletionService.js';
import { LanguageWorkerWireClient } from '../../common/languages/languageWorkerWire.js';
import { type SyntaxWorkerFactory } from '../../common/languages/syntax/syntaxService.js';
import { SyntaxModuleWorkerClient } from '../../common/languages/syntax/syntaxModuleWorkerClient.js';
import { VersionedEditorWorkerClient, type VersionedEditorWorkerFactory } from './editorWorkerService.js';
import { editorWorkerWireCodec } from '../../common/services/editorWorkerWire.js';
import { NullRenameSymbolTrackerService } from './renameSymbolTrackerService.js';
import { OpenerService } from './openerService.js';

export function registerEditorBrowserContributions(): void {
	registerTextEditorCapabilityContribution(MarkerDecorationsContribution);
}

class BrowserCodeEditorService extends AbstractCodeEditorService {
	private activeEditor: import('../editorBrowser.js').ICodeEditor | null = null;

	constructor() {
		super();
		this._register(this.onCodeEditorAdd(editor => this.activeEditor = editor));
		this._register(this.onCodeEditorRemove(editor => {
			if (this.activeEditor === editor) {
				this.activeEditor = this.listCodeEditors().at(-1) ?? null;
			}
		}));
	}

	getActiveCodeEditor(): import('../editorBrowser.js').ICodeEditor | null {
		return this.getFocusedCodeEditor() ?? this.activeEditor;
	}
}

export interface EditorBrowserServices {
	readonly codeEditorService: BrowserCodeEditorService;
	readonly workers: {
		readonly editorWorkerFactory: VersionedEditorWorkerFactory;
		readonly syntaxWorkerFactory: SyntaxWorkerFactory;
		readonly completionWorkerFactory: LanguageCompletionWorkerFactory;
	};
	readonly renameSymbolTrackerService: NullRenameSymbolTrackerService;
	readonly openerService: OpenerService;
}

/** Creates the browser-owned editor services used by a composition root. */
export function createEditorBrowserServices(): EditorBrowserServices {
	const codeEditorService = new BrowserCodeEditorService();
	return Object.freeze({
		codeEditorService,
		workers: createWorkers(),
		renameSymbolTrackerService: new NullRenameSymbolTrackerService(),
		openerService: new OpenerService(codeEditorService),
	});
}

function createWorkers(): EditorBrowserServices['workers'] {
	return Object.freeze({
		editorWorkerFactory: (model) => new VersionedEditorWorkerClient(
			model,
			() => new LanguageWorkerWireClient(
				new BrowserWorkerClientPort(new Worker(new URL('../../common/services/editorWebWorkerMain.ts', import.meta.url), { type: 'module', name: 'zeta-editor' })),
				editorWorkerWireCodec,
			),
		),
		syntaxWorkerFactory: () => new SyntaxModuleWorkerClient(
			new BrowserWorkerClientPort(new Worker(new URL('../../common/services/syntaxWorkerMain.ts', import.meta.url), { type: 'module', name: 'zeta-syntax' })),
			{ requiredProviderModules: ['language.lexical'] },
		),
		completionWorkerFactory: () => new LanguageCompletionCatalogWorkerClient(
			new BrowserWorkerClientPort(new Worker(new URL('../../common/services/languageCompletionWorkerMain.ts', import.meta.url), { type: 'module', name: 'zeta-completion' })),
			{ requiredProviderModules: ['language.word'] },
		),
	});
}
