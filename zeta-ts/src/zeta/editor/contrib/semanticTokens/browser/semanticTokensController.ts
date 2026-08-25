import { isCancellationError } from "../../../../base/common/cancellation.js";
import { type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type SemanticTokensService } from "../common/semanticTokens.js";

/** Refreshes full semantic tokens while the document and provider set remain current. */
export class SemanticTokensController extends DisposableOwner {
	private requestGeneration = 0;

	constructor(
		private readonly semanticTokensService: SemanticTokensService,
		private readonly languageId: string,
		whenLanguageSupportReady: () => Promise<unknown>,
		onDidChangeLanguageSupport: Event<void> | undefined,
		private readonly handleLanguageError: (error: unknown) => void,
	) {
		super();
		const scheduleTokens = () => {
			const requestGeneration = ++this.requestGeneration;
			queueMicrotask(() => void this.requestTokens(requestGeneration, whenLanguageSupportReady));
		};
		this.own(semanticTokensService.tokens.textModel.onDidChange(scheduleTokens));
		if (onDidChangeLanguageSupport) this.own(onDidChangeLanguageSupport(scheduleTokens));
		this.defer(() => {
			this.requestGeneration += 1;
		});
		scheduleTokens();
	}

	private async requestTokens(requestGeneration: number, whenLanguageSupportReady: () => Promise<unknown>): Promise<void> {
		try {
			await whenLanguageSupportReady();
			if (this.isDisposed || requestGeneration !== this.requestGeneration) return;
			await this.semanticTokensService.requestTokens(this.languageId);
		} catch (error) {
			if (this.isDisposed || requestGeneration !== this.requestGeneration || isCancellationError(error) || (error instanceof Error && error.name === "AbortError")) return;
			this.handleLanguageError(error);
		}
	}
}
