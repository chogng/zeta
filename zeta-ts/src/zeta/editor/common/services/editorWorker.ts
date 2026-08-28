import { Disposable, type IDisposable } from '../../../base/common/lifecycle.js';
import { TextRange, type TextEdit } from '../core/text.js';
import { type LanguageWorker } from '../languages/languageRequestCoordinator.js';
import { LanguageRequestCoordinator, LanguageRequestStatus } from '../languages/languageRequestCoordinator.js';
import { type InplaceReplaceResult } from '../languages/supports/inplaceReplaceSupport.js';
import { type TextModel } from '../model/textModel.js';
import { type UnicodeHighlight } from './unicodeTextModelHighlighter.js';

export const EDITOR_WORKER_UNICODE_HIGHLIGHTS_LANE = 'unicodeHighlights';
export const EDITOR_WORKER_MINIMAL_EDITS_LANE = 'minimalEdits';
export const EDITOR_WORKER_NAVIGATE_VALUE_LANE = 'navigateValue';

export type EditorWorkerLane = typeof EDITOR_WORKER_UNICODE_HIGHLIGHTS_LANE | typeof EDITOR_WORKER_MINIMAL_EDITS_LANE | typeof EDITOR_WORKER_NAVIGATE_VALUE_LANE;

export interface EditorWorkerUnicodeHighlightsRequest {}

export interface EditorWorkerMinimalEditsRequest {
	readonly edits: readonly TextEdit[];
}

export interface EditorWorkerNavigateValueRequest {
	readonly range: TextRange;
	readonly up: boolean;
	readonly wordDefinition: RegExp;
}

export type EditorWorkerRequest = EditorWorkerUnicodeHighlightsRequest | EditorWorkerMinimalEditsRequest | EditorWorkerNavigateValueRequest;
export type EditorWorkerResult = readonly UnicodeHighlight[] | readonly TextEdit[] | InplaceReplaceResult | undefined;
export type EditorWorkerImplementation = LanguageWorker<EditorWorkerLane, EditorWorkerRequest, EditorWorkerResult>;
export type EditorWorkerImplementationFactory = () => EditorWorkerImplementation;
export type EditorWorkerFactory = (model: TextModel) => IEditorWorkerClient;

export interface IEditorWorkerClient extends IDisposable {
	computeUnicodeHighlights(signal?: AbortSignal): Promise<readonly UnicodeHighlight[] | undefined>;
	computeMoreMinimalEdits(edits: readonly TextEdit[], signal?: AbortSignal): Promise<readonly TextEdit[] | undefined>;
	navigateValueSet(range: TextRange, up: boolean, wordDefinition: RegExp, signal?: AbortSignal): Promise<InplaceReplaceResult | undefined>;
}

/** Owns version gating and one reusable worker implementation for a TextModel. */
export class EditorWorkerClient extends Disposable implements IEditorWorkerClient {
	private readonly coordinator: LanguageRequestCoordinator<EditorWorkerLane, EditorWorkerRequest, EditorWorkerResult>;

	constructor(model: TextModel, createWorker: EditorWorkerImplementationFactory) {
		super();
		this.coordinator = this._register(new LanguageRequestCoordinator(model, createWorker));
	}

	public computeUnicodeHighlights(signal?: AbortSignal): Promise<readonly UnicodeHighlight[] | undefined> {
		return this.run(EDITOR_WORKER_UNICODE_HIGHLIGHTS_LANE, Object.freeze({}), signal) as Promise<readonly UnicodeHighlight[] | undefined>;
	}

	public computeMoreMinimalEdits(edits: readonly TextEdit[], signal?: AbortSignal): Promise<readonly TextEdit[] | undefined> {
		return this.run(EDITOR_WORKER_MINIMAL_EDITS_LANE, Object.freeze({ edits: Object.freeze([...edits]) }), signal) as Promise<readonly TextEdit[] | undefined>;
	}

	public navigateValueSet(range: TextRange, up: boolean, wordDefinition: RegExp, signal?: AbortSignal): Promise<InplaceReplaceResult | undefined> {
		return this.run(EDITOR_WORKER_NAVIGATE_VALUE_LANE, Object.freeze({ range, up, wordDefinition }), signal) as Promise<InplaceReplaceResult | undefined>;
	}

	private async run(lane: EditorWorkerLane, payload: EditorWorkerRequest, signal?: AbortSignal): Promise<EditorWorkerResult> {
		let value: EditorWorkerResult;
		const outcome = await this.coordinator.runLatest(lane, payload, result => {
			value = result.value;
		}, signal ? { signal } : {});
		return outcome.status === LanguageRequestStatus.Applied ? value : undefined;
	}
}
