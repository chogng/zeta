import { ResourceMap } from '../../../../base/common/map.js';
import { Disposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { type URI } from '../../../../base/common/uri.js';
import { CancellationTokenSource, type CancellationToken } from '../../../../base/common/cancellation.js';
import { type EditorCapability, registerEditorContribution } from '../../../browser/editorExtensions.js';
import { type EditorView } from '../../../browser/view.js';
import { createStanzaDecorationSource } from '../../../browser/viewparts/decorations/decorationPresentation.js';
import { TextSelection, TextSelectionSet } from '../../../common/core/selection.js';
import { type TextPosition, type TextRange } from '../../../common/core/text.js';
import { EditorSelectionChangeReason, type EditorSelectionChange, type EditorSelectionController } from '../../../common/cursor/editorSelectionController.js';
import { getWordSelectionRange } from '../../../common/cursor/wordBoundary.js';
import { DocumentHighlightKind, type DocumentHighlight, type DocumentHighlightProvider, type DocumentHighlightRequest, type DocumentHighlightTarget, type MultiDocumentHighlightProvider } from '../../../common/languages/documentHighlights.js';
import { type LanguageFeatureProviderMetadata, type LanguageFeatureProviderRegistry } from '../../../common/languages/languageFeatureRegistry.js';
import { TextDecorationCollection } from '../../../common/model/decorationCollection.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { TrackedRangeStickiness } from '../../../common/model/trackedRange.js';
import { type ILanguageFeaturesService } from '../../../common/services/languageService.js';
import { getHighlightDecorationOptions } from './highlightDecorations.js';
import { TextualMultiDocumentHighlightFeature } from './textualHighlightProvider.js';

type OccurrencesHighlightMode = 'off' | 'singleFile' | 'multiFile';

interface WordHighlighterOptions {
	readonly resource: URI;
	readonly languageId: string;
	readonly languageFeaturesService: ILanguageFeaturesService;
	readonly mode?: OccurrencesHighlightMode;
	readonly delay?: number;
	readonly wordPattern?: () => RegExp | undefined;
	readonly onError?: (error: unknown) => void;
}

/** Owns semantic word highlights and their editor-local lifecycle. */
class WordHighlighter extends Disposable {
	private readonly resource: URI;
	private readonly languageId: string;
	private readonly mode: OccurrencesHighlightMode;
	private readonly delay: number;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private readonly onError: (error: unknown) => void;
	private readonly providers: LanguageFeatureProviderRegistry<DocumentHighlightProvider>;
	private readonly multiDocumentProviders: LanguageFeatureProviderRegistry<MultiDocumentHighlightProvider>;
	private readonly coordinator: WordHighlightCoordinator;
	private request: CancellationTokenSource | undefined;
	private timer: ReturnType<typeof setTimeout> | undefined;
	private requestId = 0;
	private lastDecorationKey = '';
	private focused = false;
	private changingSelection = false;

	constructor(
		private readonly view: EditorView,
		private readonly selections: EditorSelectionController,
		private readonly decorations: TextDecorationCollection<DocumentHighlightKind | undefined>,
		options: WordHighlighterOptions,
	) {
		super();
		validateControllerDependencies(view, selections, decorations, options);
		this.resource = options.resource;
		this.languageId = options.languageId;
		this.mode = options.mode ?? 'singleFile';
		this.delay = options.delay ?? 250;
		this.wordPattern = options.wordPattern;
		this.onError = options.onError ?? reportHighlightError;
		this.providers = options.languageFeaturesService.documentHighlightProvider;
		this.multiDocumentProviders = options.languageFeaturesService.multiDocumentHighlightProvider;
		this.coordinator = acquireCoordinator(options.languageFeaturesService, this);
		this._register(toDisposable(() => {
			this.cancelRequest();
			this.coordinator.remove(this);
		}));
		this._register(selections.onDidChange(change => this.handleSelectionChange(change)));
		this._register(selections.textModel.onDidChange(() => this.handleModelChange()));
		this._register(view.editContext.onDidFocus(() => this.handleFocus(true)));
		this._register(view.editContext.onDidBlur(() => this.handleFocus(false)));
		this._register(view.onWillKeydown(event => this.handleKeydown(event)));
	}

	get highlightMode(): OccurrencesHighlightMode {
		return this.mode;
	}

	get documentResource(): URI {
		return this.resource;
	}

	get documentLanguageId(): string {
		return this.languageId;
	}

	get textModel(): TextModel {
		return this.selections.textModel;
	}

	get isFocused(): boolean {
		return this.focused;
	}

	get currentWordPattern(): RegExp | undefined {
		return this.wordPattern?.();
	}

	get documentHighlightProvider(): LanguageFeatureProviderRegistry<DocumentHighlightProvider> {
		return this.providers;
	}

	get multiDocumentHighlightProvider(): LanguageFeatureProviderRegistry<MultiDocumentHighlightProvider> {
		return this.multiDocumentProviders;
	}

	trigger(): void {
		if (this.mode === 'off') return;
		this.schedule(0);
	}

	moveNext(): boolean {
		return this.move(1);
	}

	movePrevious(): boolean {
		return this.move(-1);
	}

	clearHighlights(): void {
		this.cancelRequest();
		this.replaceHighlights(Object.freeze([]));
	}

	createTarget(): DocumentHighlightTarget {
		const wordPattern = this.currentWordPattern;
		return Object.freeze({
			resource: this.resource,
			model: this.textModel,
			snapshot: this.textModel.createSnapshot(),
			languageId: this.languageId,
			...(wordPattern ? { wordPattern } : {}),
		});
	}

	applyHighlights(highlights: readonly DocumentHighlight[]): void {
		this.replaceHighlights(highlights);
	}

	hasDecorations(): boolean {
		return this.decorations.size > 0;
	}

	private handleSelectionChange(change: EditorSelectionChange): void {
		if (this.changingSelection) return;
		this.cancelRequest();
		this.coordinator.clear();
		if (change.reason === EditorSelectionChangeReason.Explicit || change.reason === EditorSelectionChangeReason.CursorOperation || change.reason === EditorSelectionChangeReason.CursorUndo) this.schedule();
	}

	private handleModelChange(): void {
		this.cancelRequest();
		this.coordinator.clear();
	}

	private handleFocus(focused: boolean): void {
		this.focused = focused;
		if (focused) {
			this.schedule();
			return;
		}
		this.coordinator.clearWhenUnfocused();
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.key !== 'F7' || event.altKey || event.ctrlKey || event.metaKey) return;
		const moved = event.shiftKey ? this.movePrevious() : this.moveNext();
		if (!moved) return;
		event.preventDefault();
		event.stopPropagation();
	}

	private schedule(delay = this.delay): void {
		this.cancelRequest();
		if (this.mode === 'off' || (!this.focused && delay !== 0)) return;
		this.timer = setTimeout(() => {
			this.timer = undefined;
			void this.run();
		}, delay);
	}

	private async run(): Promise<void> {
		const position = this.highlightPosition();
		if (!position) {
			this.coordinator.clear();
			return;
		}
		const request = new CancellationTokenSource();
		this.request = request;
		const requestId = ++this.requestId;
		try {
			const result = await this.coordinator.provide(this, position, request.token);
			if (request.token.isCancellationRequested || requestId !== this.requestId) return;
			this.coordinator.apply(this, result);
		} catch (error) {
			if (!request.token.isCancellationRequested) this.onError(error);
		} finally {
			if (this.request === request) this.request = undefined;
			request.dispose();
		}
	}

	private highlightPosition(): TextPosition | undefined {
		if (this.selections.selections.selections.length !== 1) return undefined;
		const selection = this.selections.selections.primary;
		if (!selectionFitsModel(this.textModel, selection.range) || selection.range.start.lineIndex !== selection.range.end.lineIndex) return undefined;
		const range = getWordSelectionRange(this.textModel, selection.range.start, this.currentWordPattern);
		if (range.empty || range.start.compareTo(selection.range.start) > 0 || range.end.compareTo(selection.range.end) < 0) return undefined;
		return selection.range.start;
	}

	private replaceHighlights(highlights: readonly DocumentHighlight[]): void {
		const key = highlights.map(highlight => `${this.textModel.offsetAt(highlight.range.start)}-${this.textModel.offsetAt(highlight.range.end)}:${highlight.kind ?? ''}`).join(',');
		if (key === this.lastDecorationKey) return;
		this.lastDecorationKey = key;
		this.decorations.replaceAll(highlights.map(highlight => ({
			range: highlight.range,
			stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
			metadata: highlight.kind,
		})));
	}

	private move(direction: 1 | -1): boolean {
		const ranges = [...this.decorations.decorations].map(decoration => decoration.range).sort((left, right) => left.start.compareTo(right.start));
		if (ranges.length === 0) return false;
		const activeOffset = this.textModel.offsetAt(this.selections.selections.primary.active);
		const currentIndex = ranges.findIndex(range => this.textModel.offsetAt(range.start) <= activeOffset && this.textModel.offsetAt(range.end) >= activeOffset);
		const nextIndex = direction === 1 ? (currentIndex + 1) % ranges.length : (currentIndex - 1 + ranges.length) % ranges.length;
		const destination = ranges[nextIndex]!;
		this.changingSelection = true;
		try {
			this.selections.setCursorSelections(TextSelectionSet.single(TextSelection.collapsedAt(destination.start)));
			this.view.revealPosition(destination.start);
		} finally {
			this.changingSelection = false;
		}
		return true;
	}

	private cancelRequest(): void {
		if (this.timer !== undefined) {
			clearTimeout(this.timer);
			this.timer = undefined;
		}
		this.request?.cancel();
		this.request?.dispose();
		this.request = undefined;
		this.requestId += 1;
	}
}

export async function getOccurrencesAtPosition(registry: LanguageFeatureProviderRegistry<DocumentHighlightProvider>, model: DocumentHighlightTarget, position: TextPosition, token: CancellationToken): Promise<ResourceMap<readonly DocumentHighlight[]>> {
	const request = createDocumentHighlightRequest(model, position);
	for (const provider of orderedProviders(registry, model.languageId)) {
		if (!isDocumentHighlightRequestCurrent(request, token)) return new ResourceMap();
		const highlights = await provider.provideDocumentHighlights(request, token);
		if (!isDocumentHighlightRequestCurrent(request, token)) return new ResourceMap();
		if (highlights === undefined || highlights === null) continue;
		const result = new ResourceMap<readonly DocumentHighlight[]>();
		result.set(model.resource, normalizeHighlights(model.model, highlights));
		return result;
	}
	return new ResourceMap();
}

export async function getOccurrencesAcrossMultipleModels(registry: LanguageFeatureProviderRegistry<MultiDocumentHighlightProvider>, model: DocumentHighlightTarget, position: TextPosition, token: CancellationToken, otherModels: readonly DocumentHighlightTarget[]): Promise<ResourceMap<readonly DocumentHighlight[]>> {
	const targets = Object.freeze([model, ...otherModels]);
	const request = createDocumentHighlightRequest(model, position);
	for (const provider of orderedProviders(registry, model.languageId)) {
		if (!isDocumentHighlightRequestCurrent(request, token, targets)) return new ResourceMap();
		const highlights = await provider.provideMultiDocumentHighlights(request, targets, token);
		if (!isDocumentHighlightRequestCurrent(request, token, targets)) return new ResourceMap();
		if (highlights !== undefined && highlights !== null) return normalizeHighlightMap(highlights, targets);
	}
	return new ResourceMap();
}

function createDocumentHighlightRequest(model: DocumentHighlightTarget, position: TextPosition): DocumentHighlightRequest {
	return Object.freeze({ ...model, position });
}

function orderedProviders<TProvider extends LanguageFeatureProviderMetadata>(registry: LanguageFeatureProviderRegistry<TProvider>, languageId: string): readonly TProvider[] {
	return Object.freeze([...registry.getProviders(languageId)].sort((left, right) => Number(left.languageIds.includes('*')) - Number(right.languageIds.includes('*'))));
}

function isDocumentHighlightRequestCurrent(request: DocumentHighlightRequest, token: CancellationToken, targets: readonly DocumentHighlightTarget[] = []): boolean {
	return !token.isCancellationRequested && !request.model.isDisposed && request.model.version === request.snapshot.version && targets.every(target => !target.model.isDisposed && target.model.version === target.snapshot.version);
}

function normalizeHighlightMap(result: ReadonlyMap<URI, readonly DocumentHighlight[]>, targets: readonly DocumentHighlightTarget[]): ResourceMap<readonly DocumentHighlight[]> {
	if (!result || typeof result[Symbol.iterator] !== 'function') throw new TypeError('Multi-document highlights must be a resource map');
	const models = new ResourceMap<TextModel>();
	for (const target of targets) models.set(target.resource, target.model);
	const normalized = new ResourceMap<readonly DocumentHighlight[]>();
	for (const [resource, highlights] of result) {
		const model = models.get(resource);
		if (!model) throw new RangeError(`Document highlights returned an unknown resource '${resource.toString()}'`);
		normalized.set(resource, normalizeHighlights(model, highlights));
	}
	return normalized;
}

function normalizeHighlights(model: TextModel, highlights: readonly DocumentHighlight[]): readonly DocumentHighlight[] {
	if (!Array.isArray(highlights)) throw new TypeError('Document highlights must be an array');
	return Object.freeze(highlights.map(highlight => {
		if (!highlight || typeof highlight !== 'object' || !highlight.range) throw new TypeError('Document highlight must contain a range');
		model.offsetAt(highlight.range.start);
		model.offsetAt(highlight.range.end);
		if (highlight.kind !== undefined && !Object.values(DocumentHighlightKind).includes(highlight.kind)) throw new TypeError('Document highlight kind is invalid');
		return Object.freeze({ range: highlight.range, ...(highlight.kind ? { kind: highlight.kind } : {}) });
	}));
}

class WordHighlightCoordinator {
	private readonly controllers = new Set<WordHighlighter>();
	private clearTimer: ReturnType<typeof setTimeout> | undefined;

	constructor(private readonly service: ILanguageFeaturesService) {}

	add(controller: WordHighlighter): void {
		this.controllers.add(controller);
	}

	remove(controller: WordHighlighter): void {
		this.controllers.delete(controller);
		if (this.controllers.size === 0) coordinators.delete(this.service);
	}

	clear(): void {
		for (const controller of this.controllers) controller.clearHighlights();
	}

	clearWhenUnfocused(): void {
		if (this.clearTimer !== undefined) clearTimeout(this.clearTimer);
		this.clearTimer = setTimeout(() => {
			this.clearTimer = undefined;
			if (![...this.controllers].some(controller => controller.isFocused)) this.clear();
		}, 0);
	}

	async provide(source: WordHighlighter, position: TextPosition, token: CancellationToken): Promise<ResourceMap<readonly DocumentHighlight[]>> {
		const targets = source.highlightMode === 'multiFile'
			? [...this.controllers].filter(controller => controller.highlightMode === 'multiFile').map(controller => controller.createTarget())
			: [source.createTarget()];
		const primary = source.createTarget();
		if (targets.length > 1 && source.multiDocumentHighlightProvider.getProviders(source.documentLanguageId).length > 0) {
			return getOccurrencesAcrossMultipleModels(source.multiDocumentHighlightProvider, primary, position, token, targets.filter(target => target.model !== primary.model));
		}
		return getOccurrencesAtPosition(source.documentHighlightProvider, primary, position, token);
	}

	apply(source: WordHighlighter, result: ResourceMap<readonly DocumentHighlight[]>): void {
		for (const controller of this.controllers) {
			const highlights = result.get(controller.documentResource);
			controller.applyHighlights(highlights ?? Object.freeze([]));
			if (source.highlightMode === 'singleFile' && controller !== source) controller.clearHighlights();
		}
	}
}

const coordinators = new WeakMap<ILanguageFeaturesService, WordHighlightCoordinator>();

function acquireCoordinator(service: ILanguageFeaturesService, controller: WordHighlighter): WordHighlightCoordinator {
	let coordinator = coordinators.get(service);
	if (!coordinator) {
		coordinator = new WordHighlightCoordinator(service);
		coordinators.set(service, coordinator);
	}
	coordinator.add(controller);
	return coordinator;
}

function validateControllerDependencies(view: EditorView, selections: EditorSelectionController, decorations: TextDecorationCollection<DocumentHighlightKind | undefined>, options: WordHighlighterOptions): void {
	if (view.viewport.textModel !== selections.textModel || selections.textModel !== decorations.textModel) throw new TypeError('Word highlighter dependencies must share one text model');
	if (!options || typeof options !== 'object' || !options.resource || !options.languageId || !options.languageFeaturesService) throw new TypeError('Word highlighter requires resource and language services');
	if (options.mode !== undefined && options.mode !== 'off' && options.mode !== 'singleFile' && options.mode !== 'multiFile') throw new TypeError('Word highlighter mode is invalid');
	if (options.delay !== undefined && (!Number.isSafeInteger(options.delay) || options.delay < 0 || options.delay > 2_000)) throw new RangeError('Word highlighter delay must be an integer between 0 and 2000');
	if (options.wordPattern !== undefined && typeof options.wordPattern !== 'function') throw new TypeError('Word highlighter word pattern resolver must be a function');
	if (options.onError !== undefined && typeof options.onError !== 'function') throw new TypeError('Word highlighter error handler must be a function');
}

function selectionFitsModel(model: WordHighlighter['textModel'], range: TextRange): boolean {
	return positionFitsModel(model, range.start.lineIndex, range.start.columnIndex) && positionFitsModel(model, range.end.lineIndex, range.end.columnIndex);
}

function positionFitsModel(model: WordHighlighter['textModel'], lineIndex: number, columnIndex: number): boolean {
	return Number.isSafeInteger(lineIndex) && Number.isSafeInteger(columnIndex) && lineIndex >= 0 && columnIndex >= 0 && lineIndex < model.lineCount && columnIndex <= model.getLineLength(lineIndex);
}

export class WordHighlighterContribution extends Disposable {
	static readonly ID = 'editor.contrib.wordHighlighter';
	private readonly wordHighlighter: WordHighlighter;

	constructor(view: EditorView, selections: EditorSelectionController, decorations: TextDecorationCollection<DocumentHighlightKind | undefined>, options: WordHighlighterOptions) {
		super();
		this.wordHighlighter = this._register(new WordHighlighter(view, selections, decorations, options));
	}

	public saveViewState(): boolean {
		return this.wordHighlighter.hasDecorations();
	}

	public moveNext(): void {
		this.wordHighlighter.moveNext();
	}

	public moveBack(): void {
		this.wordHighlighter.movePrevious();
	}

	public restoreViewState(state: boolean | undefined): void {
		if (state) this.wordHighlighter.trigger();
	}

	public stopHighlighting(): void {
		this.wordHighlighter.clearHighlights();
	}
}

function reportHighlightError(error: unknown): void {
	console.error('Document highlight request failed', error);
}

const occurrenceDecorations: EditorCapability<TextDecorationCollection<DocumentHighlightKind | undefined>> = Object.freeze({ id: 'editor.capability.occurrenceDecorations' });

registerEditorContribution({
	id: WordHighlighterContribution.ID,
	configure: context => {
		const decorations = context.register(new TextDecorationCollection<DocumentHighlightKind | undefined>(context.model));
		context.provideCapability(occurrenceDecorations, decorations);
		context.addDecorationSource(createStanzaDecorationSource(decorations, decoration => getHighlightDecorationOptions(decoration.metadata)));
		context.register(new TextualMultiDocumentHighlightFeature(context.languageFeaturesService));
	},
	install: context => {
		if (context.kind !== 'text' || context.model.largeFile.tooLargeForTokenization) return;
		context.register(new WordHighlighterContribution(context.view, context.selections, context.getCapability(occurrenceDecorations), {
			resource: context.options.input.resource,
			languageId: context.languageId,
			languageFeaturesService: context.languageFeaturesService,
			mode: context.options.occurrencesHighlight,
			delay: context.options.occurrencesHighlightDelay,
			wordPattern: () => context.configurations.getLanguageConfiguration(context.languageId).wordPattern,
			onError: context.onLanguageError,
		}));
	},
});
