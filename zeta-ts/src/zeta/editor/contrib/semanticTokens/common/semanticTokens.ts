import { Disposable } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { LanguageFeatureRegistry } from "../../../common/languageFeatureRegistry.js";
import { LanguageRequestCoordinator, type LanguageRequestOptions, type LanguageRequestOutcome, type LanguageWorker, type LanguageWorkerRequest } from "../../../common/languages/languageRequestCoordinator.js";
import { LanguageResultAcceptance } from "../../../common/languages/languageResultStore.js";
import { createLanguageTokenStore, type LanguageToken, type LanguageTokenResult } from "../../../common/tokens/languageTokens.js";
import { type TextModel } from "../../../common/model/textModel.js";
import type { SemanticTokenModelSource, SemanticTokenStylingResolver } from '../../../common/services/resolvedSemanticTokens.js';
import { type ISemanticTokensStylingService } from '../../../common/services/semanticTokensStyling.js';

export const SEMANTIC_TOKENS_LANE = "semanticTokens";
export type SemanticTokensLane = typeof SEMANTIC_TOKENS_LANE;

export interface LanguageSemanticTokensRequest {
	readonly requestId: number;
	readonly model: TextModel;
	readonly snapshot: ReturnType<TextModel['createVersionedSnapshot']>;
	readonly languageId: string;
	readonly resource?: URI;
}

export interface LanguageSemanticTokensProvider {
	provideSemanticTokens(request: LanguageSemanticTokensRequest, signal: AbortSignal): LanguageTokenResult | undefined | PromiseLike<LanguageTokenResult | undefined>;
}

export interface SemanticTokensModelPart extends SemanticTokenModelSource {
	readonly styling: SemanticTokenStylingResolver;
}

interface SemanticTokensPayload {
	readonly languageId: string;
	readonly resource?: URI;
}

/** Runs full-document semantic-token providers through the editor's version gate. */
export class SemanticTokensService extends Disposable {
	readonly tokens: ReturnType<typeof createLanguageTokenStore>;
	readonly styling: SemanticTokenStylingResolver;
	private readonly coordinator: LanguageRequestCoordinator<SemanticTokensLane, SemanticTokensPayload, SemanticTokensProviderResult>;
	private providerStyling: SemanticTokenStylingResolver | undefined;

	constructor(
		model: TextModel,
		providers: LanguageFeatureRegistry<LanguageSemanticTokensProvider>,
		private readonly stylingService: ISemanticTokensStylingService,
		private readonly resource?: URI,
	) {
		super();
		this.tokens = this._register(createLanguageTokenStore(model));
		this.styling = Object.freeze({
			resolve: (token: LanguageToken) => {
				if (!this.providerStyling) throw new ReferenceError('Semantic token styling has no active provider');
				return this.providerStyling.resolve(token);
			},
		});
		this.coordinator = this._register(new LanguageRequestCoordinator(model, () => new SemanticTokensProviderWorker(model, providers)));
	}

	requestTokens(languageId: string, options: LanguageRequestOptions = {}): Promise<LanguageRequestOutcome> {
		const payload = Object.freeze({ languageId, ...(this.resource ? { resource: this.resource } : {}) });
		return this.coordinator.runLatest(SEMANTIC_TOKENS_LANE, payload, result => {
			this.providerStyling = result.value.provider ? this.stylingService.getStyling(result.value.provider) : undefined;
			const acceptance = this.tokens.accept({ ...result, value: result.value.tokens });
			if (acceptance !== LanguageResultAcceptance.Applied) throw new Error(`Semantic-token result store rejected current result as '${acceptance}'`);
		}, options);
	}
}

interface SemanticTokensProviderResult {
	readonly tokens: LanguageTokenResult;
	readonly provider?: LanguageSemanticTokensProvider;
}

class SemanticTokensProviderWorker implements LanguageWorker<SemanticTokensLane, SemanticTokensPayload, SemanticTokensProviderResult> {
	constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureRegistry<LanguageSemanticTokensProvider>) {}

	async run(request: LanguageWorkerRequest<SemanticTokensLane, SemanticTokensPayload>, signal: AbortSignal): Promise<SemanticTokensProviderResult> {
		const providerRequest = Object.freeze({ requestId: request.requestId, model: this.model, snapshot: request.snapshot, languageId: request.payload.languageId, ...(request.payload.resource ? { resource: request.payload.resource } : {}) });
		for (const provider of this.providers.ordered(this.model)) {
			signal.throwIfAborted();
			const result = await provider.provideSemanticTokens(providerRequest, signal);
			signal.throwIfAborted();
			if (result) return Object.freeze({ tokens: result, provider });
		}
		return Object.freeze({ tokens: EMPTY_RESULT });
	}

	dispose(): void {}
	[Symbol.dispose](): void { this.dispose(); }
}

const EMPTY_RESULT: LanguageTokenResult = Object.freeze({ tokens: Object.freeze([]) });
