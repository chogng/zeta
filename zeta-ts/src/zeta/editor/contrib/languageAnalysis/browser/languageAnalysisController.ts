import { isCancellationError } from "../../../../base/common/errors.js";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { type Event } from "../../../../base/common/event.js";
import { type SyntaxService } from "../../../common/languages/syntax/syntaxService.js";

/** Schedules syntax lanes while the selected Workbench mode's language support changes. */
export class LanguageAnalysisController extends Disposable {
	private requestGeneration = 0;

	constructor(
		private readonly syntaxService: SyntaxService,
		private readonly languageId: string,
		whenLanguageSupportReady: () => Promise<unknown>,
		onDidChangeLanguageSupport: Event<void> | undefined,
		private readonly handleLanguageError: (error: unknown) => void,
	) {
		super();
		const scheduleAnalysis = () => {
			const requestGeneration = ++this.requestGeneration;
			queueMicrotask(() => void this.requestAnalysis(requestGeneration, whenLanguageSupportReady));
		};
		this._register(syntaxService.tokens.textModel.onDidChange(scheduleAnalysis));
		if (onDidChangeLanguageSupport) this._register(onDidChangeLanguageSupport(scheduleAnalysis));
		this._register(toDisposable(() => {
			this.requestGeneration += 1;
		}));
		scheduleAnalysis();
	}

	private async requestAnalysis(requestGeneration: number, whenLanguageSupportReady: () => Promise<unknown>): Promise<void> {
		try {
			await whenLanguageSupportReady();
			if (this.isDisposed || requestGeneration !== this.requestGeneration) return;
			await this.syntaxService.requestAll(this.languageId);
		} catch (error) {
			if (this.isDisposed || requestGeneration !== this.requestGeneration || isCancellationError(error) || (error instanceof Error && error.name === "AbortError")) return;
			this.handleLanguageError(error);
		}
	}
}
