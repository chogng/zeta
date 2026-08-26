import { DisposableOwner } from '../../../base/common/lifecycle.js';
import { type EditorSelectionController } from '../../common/cursor/editorSelectionController.js';
import { createLanguageCompletionIncompleteRefreshContext, type LanguageCompletionContext } from '../../common/languages/completion/languageCompletionProviders.js';
import { type EditorViewport } from '../view/editorViewport.js';
import { type InputCompletionOptions, type InputCompletionRequestDelegate, type InputCompletionRequests, type InputCompletionSession, type InputCompletionView } from './inputContracts.js';

/**
 * Owns completion requests and the browser presentation for one editor input.
 * The common completion session remains owned by the Suggest contribution.
 */
export class InputCompletionController extends DisposableOwner implements InputCompletionRequestDelegate {
	readonly session: InputCompletionSession;
	readonly requests: InputCompletionRequests | undefined;
	readonly widget: InputCompletionView;

	private completionRequest: AbortController | undefined;
	private completionIsIncomplete = false;

	constructor(
		private readonly input: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
		options: InputCompletionOptions,
	) {
		super();
		this.session = options.session;
		this.requests = options.requests;
		if (typeof options.viewFactory !== 'function') {
			this.dispose();
			throw new TypeError('Text input completion requires a view factory');
		}
		const completionResults = this.requests?.service.results;
		if (completionResults) {
			this.completionIsIncomplete = completionResults.result?.value.isIncomplete === true;
			this.own(completionResults.onDidChange(change => {
				if (change.result) this.completionIsIncomplete = change.result.value.isIncomplete;
			}));
		}
		this.widget = this.own(options.viewFactory(this.input, this.viewport, this.selectionController, this.session));
		this.defer(() => this.cancelCompletionRequest());
	}

	readIsIncomplete(): boolean {
		const result = this.requests?.service.results.result;
		if (result) return result.value.isIncomplete;
		if (this.completionIsIncomplete) return true;
		try {
			return this.session.state?.isIncomplete === true;
		} catch (error) {
			if (error instanceof ReferenceError) return false;
			throw error;
		}
	}

	requestAfterInsert(insertedText: string, refreshIncomplete: boolean): void {
		const requests = this.requests;
		if (!requests) return;
		if ([...insertedText].length === 1) {
			const selections = this.selectionController.selections;
			if (selections.selections.length !== 1 || !selections.primary.collapsed) {
				this.session.cancel();
				return;
			}
			const position = selections.primary.active;
			const modelVersion = this.viewport.textModel.version;
			const request = this.beginCompletionRequest();
			void requests.service.requestTriggerCharacter(
				requests.languageId,
				position,
				insertedText,
				{ signal: request.signal },
			).then(outcome => {
				if (
					!request.signal.aborted &&
					outcome === undefined &&
					refreshIncomplete &&
					this.viewport.textModel.version === modelVersion &&
					this.selectionController.selections.selections.length === 1 &&
					this.selectionController.selections.primary.collapsed &&
					this.selectionController.selections.primary.active.compareTo(position) === 0
				) {
					this.requestCompletion(createLanguageCompletionIncompleteRefreshContext());
				}
			}).catch(error => {
				if (!request.signal.aborted) this.reportCompletionRequestError(error);
			}).finally(() => this.releaseCompletionRequest(request));
			return;
		}
		if (refreshIncomplete) this.requestCompletion(createLanguageCompletionIncompleteRefreshContext());
	}

	requestCompletion(context: LanguageCompletionContext): void {
		const requests = this.requests;
		if (!requests) return;
		const selections = this.selectionController.selections;
		if (selections.selections.length !== 1 || !selections.primary.collapsed) {
			this.session.cancel();
			return;
		}
		const request = this.beginCompletionRequest();
		try {
			void requests.service.request(
				requests.languageId,
				selections.primary.active,
				context,
				{ signal: request.signal },
			).catch(error => {
				if (!request.signal.aborted) this.reportCompletionRequestError(error);
			}).finally(() => this.releaseCompletionRequest(request));
		} catch (error) {
			this.releaseCompletionRequest(request);
			if (!request.signal.aborted) this.reportCompletionRequestError(error);
		}
	}

	private beginCompletionRequest(): AbortController {
		this.cancelCompletionRequest();
		const request = new AbortController();
		this.completionRequest = request;
		return request;
	}

	private cancelCompletionRequest(): void {
		this.completionRequest?.abort();
		this.completionRequest = undefined;
	}

	private releaseCompletionRequest(request: AbortController): void {
		if (this.completionRequest === request) this.completionRequest = undefined;
	}

	private reportCompletionRequestError(error: unknown): void {
		try {
			const handler = this.requests?.onRequestError;
			if (handler) handler(error);
			else console.error('Stanza completion request failed', error);
		} catch (reportingError) {
			console.error('Stanza completion request and error handler both failed', new AggregateError([error, reportingError]));
		}
	}
}
