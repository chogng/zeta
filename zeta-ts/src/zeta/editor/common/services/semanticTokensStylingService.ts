import { Disposable } from '../../../base/common/lifecycle.js';
import { InstantiationType, registerSingleton } from '../../../platform/instantiation/common/extensions.js';
import { SemanticTokensProviderStyling } from './semanticTokensProviderStyling.js';
import { ISemanticTokensStylingService, type DocumentTokensProvider } from './semanticTokensStyling.js';

/** Caches the provider styling that resolves each provider's LanguageToken values. */
export class SemanticTokensStylingService extends Disposable implements ISemanticTokensStylingService {
	public readonly _serviceBrand: undefined;
	private _caches = new WeakMap<DocumentTokensProvider, SemanticTokensProviderStyling>();

	constructor() {
		super();
	}

	public getStyling(provider: DocumentTokensProvider): SemanticTokensProviderStyling {
		this.assertNotDisposed();
		let styling = this._caches.get(provider);
		if (!styling) {
			styling = new SemanticTokensProviderStyling(provider);
			this._caches.set(provider, styling);
		}
		return styling;
	}

	protected override disposeCore(): void {
		this._caches = new WeakMap();
	}
}

registerSingleton(ISemanticTokensStylingService, SemanticTokensStylingService, InstantiationType.Delayed);
