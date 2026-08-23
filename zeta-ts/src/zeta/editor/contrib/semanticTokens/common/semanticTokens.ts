import { DisposableOwner, type IDisposable } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languages/languageFeatureRegistry.js";
import { LanguageRequestCoordinator, type LanguageRequestOptions, type LanguageRequestOutcome, type LanguageWorker, type LanguageWorkerRequest } from "../../../common/languages/languageRequestCoordinator.js";
import { LanguageResultAcceptance } from "../../../common/languages/languageResultStore.js";
import { createLanguageTokenStore, type LanguageToken, type LanguageTokenResult } from "../../../common/tokens/languageTokens.js";
import { type LanguageTokenLine } from "../../../common/tokens/languageTokenLineIndex.js";
import { type TextModel } from "../../../common/model/textModel.js";

export const SEMANTIC_TOKENS_LANE = "semanticTokens";
export type SemanticTokensLane = typeof SEMANTIC_TOKENS_LANE;

export interface LanguageSemanticTokensRequest {
	readonly requestId: number;
	readonly model: TextModel;
	readonly snapshot: ReturnType<TextModel["createSnapshot"]>;
	readonly languageId: string;
	readonly resource?: URI;
}

export interface LanguageSemanticTokensProvider extends LanguageFeatureProviderMetadata {
	provideSemanticTokens(request: LanguageSemanticTokensRequest, signal: AbortSignal): LanguageTokenResult | undefined | PromiseLike<LanguageTokenResult | undefined>;
}

interface SemanticTokensPayload {
	readonly languageId: string;
	readonly resource?: URI;
}

/** Runs full-document semantic-token providers through the editor's version gate. */
export class SemanticTokensService extends DisposableOwner {
	readonly tokens: ReturnType<typeof createLanguageTokenStore>;
	private readonly coordinator: LanguageRequestCoordinator<SemanticTokensLane, SemanticTokensPayload, LanguageTokenResult>;

	constructor(model: TextModel, providers: LanguageFeatureProviderRegistry<LanguageSemanticTokensProvider>, private readonly resource?: URI) {
		super();
		this.tokens = this.own(createLanguageTokenStore(model));
		this.coordinator = this.own(new LanguageRequestCoordinator(model, () => new SemanticTokensProviderWorker(model, providers)));
	}

	requestTokens(languageId: string, options: LanguageRequestOptions = {}): Promise<LanguageRequestOutcome> {
		const payload = Object.freeze({ languageId, ...(this.resource ? { resource: this.resource } : {}) });
		return this.coordinator.runLatest(SEMANTIC_TOKENS_LANE, payload, result => {
			const acceptance = this.tokens.accept(result);
			if (acceptance !== LanguageResultAcceptance.Applied) throw new Error(`Semantic-token result store rejected current result as '${acceptance}'`);
		}, options);
	}
}

class SemanticTokensProviderWorker implements LanguageWorker<SemanticTokensLane, SemanticTokensPayload, LanguageTokenResult> {
	constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageSemanticTokensProvider>) {}

	async run(request: LanguageWorkerRequest<SemanticTokensLane, SemanticTokensPayload>, signal: AbortSignal): Promise<LanguageTokenResult> {
		const providerRequest = Object.freeze({ requestId: request.requestId, model: this.model, snapshot: request.snapshot, languageId: request.payload.languageId, ...(request.payload.resource ? { resource: request.payload.resource } : {}) });
		for (const provider of this.providers.getProviders(request.payload.languageId)) {
			signal.throwIfAborted();
			const result = await provider.provideSemanticTokens(providerRequest, signal);
			signal.throwIfAborted();
			if (result) return result;
		}
		return EMPTY_RESULT;
	}

	dispose(): void {}
	[Symbol.dispose](): void { this.dispose(); }
}

/** Stable common semantic-token source shape; browser presentation stays outside this contract. */
export interface SemanticTokensModelPart {
	readonly textModel: TextModel;
	readonly onDidChange: (listener: (...args: any[]) => void) => IDisposable;
	readonly lines: readonly LanguageTokenLine[];
	getLineTokens(lineIndex: number): readonly LanguageToken[];
}

const EMPTY_RESULT: LanguageTokenResult = Object.freeze({ tokens: Object.freeze([]) });
