import { registerEditorContribution } from '../../../browser/editorExtensions.js';
import { type EditorViewport } from '../../../browser/view.js';
import { StableEditorScrollState } from '../../../browser/stableEditorScroll.js';
import { Disposable, DisposableMap, DisposableStore, MutableDisposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { type URI } from '../../../../base/common/uri.js';
import { type LanguageFeatureProviderRegistry } from '../../../common/languageFeatureRegistry.js';
import { type LanguageCodeLensCommand, type LanguageCodeLensProvider } from '../common/codelens.js';
import { codeLensCache } from './codeLensCache.js';
import { CodeLensModel, getCodeLensModel, resolveCodeLensItem, type CodeLensItem } from './codelens.js';
import { CodeLensWidget } from './codelensWidget.js';

export type ExecuteCodeLensCommand = (id: string, args: readonly unknown[] | undefined) => void | Promise<void>;

/** Coordinates provider requests, visible deferred resolves, and line-owned CodeLens widgets. */
export class CodeLensContribution extends Disposable {
	public static readonly ID = 'editor.contrib.codelens';

	private readonly widgets = this._register(new DisposableMap<number, CodeLensWidget>());
	private readonly providerListeners = this._register(new DisposableStore());
	private readonly cacheExpiry = this._register(new MutableDisposable());
	private readonly resolvingWidgets = new Map<CodeLensWidget, AbortController>();
	private request: AbortController | undefined;
	private currentModel = CodeLensModel.Empty;
	private cachedModel: CodeLensModel | undefined;
	private refreshPromise: Promise<void> | undefined;
	private resolvePromise: Promise<void> | undefined;
	private refreshScheduled = false;
	private modelVersion: number;

	public constructor(
		private readonly viewport: EditorViewport,
		private readonly providers: LanguageFeatureProviderRegistry<LanguageCodeLensProvider>,
		private readonly languageId: string,
		private readonly resource: URI | undefined,
		private readonly onExecuteCommand: ExecuteCodeLensCommand | undefined,
		private readonly onError: (error: unknown) => void = error => console.error('Stanza CodeLens failed', error),
	) {
		super();
		this.modelVersion = viewport.textModel.version;
		this.bindProviderListeners();
		this._register(providers.onDidChange(() => {
			this.bindProviderListeners();
			this.scheduleRefresh();
		}));
		this._register(viewport.onDidChangeLayout(change => {
			if (change.layout.modelVersion !== this.modelVersion) {
				this.modelVersion = change.layout.modelVersion;
				this.currentModel = CodeLensModel.Empty;
				this.clearWidgets();
				this.scheduleRefresh();
				return;
			}
			this.layoutAndResolve();
		}));
		this._register(toDisposable(() => this.request?.abort()));
		const cachedModel = resource ? codeLensCache.get(resource, viewport.textModel.lineCount) : undefined;
		if (cachedModel) {
			this.cachedModel = cachedModel;
			this.currentModel = cachedModel;
			this.reconcileWidgets(cachedModel.lenses);
		}
		this.refreshPromise = this.refresh();
	}

	public async getModel(): Promise<CodeLensModel> {
		await this.refreshPromise;
		while (this.resolvePromise) await this.resolvePromise;
		return this.currentModel;
	}

	private bindProviderListeners(): void {
		this.providerListeners.clear();
		for (const provider of this.providers.getProviders(this.languageId)) {
			if (provider.onDidChange) this.providerListeners.add(provider.onDidChange(() => this.scheduleRefresh()));
		}
	}

	private scheduleRefresh(): void {
		if (this.refreshScheduled || this.isDisposed) return;
		this.refreshScheduled = true;
		queueMicrotask(() => {
			this.refreshScheduled = false;
			if (!this.isDisposed) this.refreshPromise = this.refresh();
		});
	}

	private async refresh(): Promise<void> {
		this.request?.abort();
		this.resolvePromise = undefined;
		this.resolvingWidgets.clear();
		this.cacheExpiry.clear();
		if (this.providers.getProviders(this.languageId).length === 0) {
			this.showCachedModelUntilExpiry();
			return;
		}
		const request = this.request = new AbortController();
		const model = await getCodeLensModel({
			model: this.viewport.textModel,
			providers: this.providers,
			languageId: this.languageId,
			resource: this.resource,
			signal: request.signal,
			onError: this.onError,
		});
		if (request.signal.aborted || request !== this.request) return;
		this.cachedModel = undefined;
		this.currentModel = model;
		this.updateCache();
		this.reconcileWidgets(model.lenses);
		this.layoutAndResolve();
	}

	private showCachedModelUntilExpiry(): void {
		const cachedModel = this.resource ? codeLensCache.get(this.resource, this.viewport.textModel.lineCount) : undefined;
		if (!cachedModel) {
			this.cachedModel = undefined;
			this.currentModel = CodeLensModel.Empty;
			this.clearWidgets();
			return;
		}
		this.cachedModel = cachedModel;
		this.currentModel = cachedModel;
		this.reconcileWidgets(cachedModel.lenses);
		const timeout = globalThis.setTimeout(() => {
			if (this.cachedModel !== cachedModel || this.isDisposed) return;
			if (this.resource) codeLensCache.delete(this.resource);
			this.cachedModel = undefined;
			this.currentModel = CodeLensModel.Empty;
			this.clearWidgets();
		}, 30_000);
		this.cacheExpiry.value = toDisposable(() => globalThis.clearTimeout(timeout));
	}

	private reconcileWidgets(items: readonly CodeLensItem[]): void {
		const scrollState = StableEditorScrollState.capture(this.viewport);
		const groups = groupCodeLensItems(items);
		const currentWidgets = new Map(this.widgets);
		try {
			for (const lineIndex of [...this.widgets.keys()]) {
				if (!groups.has(lineIndex)) this.widgets.deleteAndDispose(lineIndex);
			}
			for (const [lineIndex, lineItems] of groups) {
				const current = currentWidgets.get(lineIndex);
				if (current) {
					current.updateCodeLensItems(lineItems);
					continue;
				}
				this.widgets.set(lineIndex, new CodeLensWidget(this.viewport, lineItems, this.onExecuteCommand ? command => this.executeCommand(command) : undefined));
			}
		} finally {
			scrollState.restore(this.viewport);
		}
	}

	private clearWidgets(): void {
		const scrollState = StableEditorScrollState.capture(this.viewport);
		try {
			for (const lineIndex of [...this.widgets.keys()]) this.widgets.deleteAndDispose(lineIndex);
		} finally {
			scrollState.restore(this.viewport);
		}
	}

	private layoutAndResolve(): void {
		for (const [, widget] of this.widgets) widget.layout();
		const request = this.request;
		if (!request || request.signal.aborted) return;
		const visible = [...this.widgets].map(([, widget]) => widget).filter(widget => widget.isVisible() && widget.needsResolve && !this.resolvingWidgets.has(widget));
		if (visible.length === 0) return;
		const batchPromise = Promise.all(visible.map(widget => this.resolveWidget(widget, request))).then(
			() => undefined,
			error => { if (!request.signal.aborted) this.onError(error); },
		);
		const previousPromise = this.resolvePromise;
		const resolvePromise = previousPromise
			? Promise.all([previousPromise, batchPromise]).then(() => undefined)
			: batchPromise;
		this.resolvePromise = resolvePromise;
		void resolvePromise.finally(() => {
			if (this.resolvePromise === resolvePromise) this.resolvePromise = undefined;
		});
	}

	private async resolveWidget(widget: CodeLensWidget, request: AbortController): Promise<void> {
		this.resolvingWidgets.set(widget, request);
		const items = widget.codeLensItems;
		try {
			const symbols = await Promise.all(items.map(item => resolveCodeLensItem({
				model: this.viewport.textModel,
				languageId: this.languageId,
				resource: this.resource,
				signal: request.signal,
				onError: this.onError,
			}, item)));
			if (request.signal.aborted || request !== this.request || widget.codeLensItems !== items || widget.isDisposed) return;
			const resolvedItems = items.map((item, index) => symbols[index] ? Object.freeze({ symbol: symbols[index]!, provider: item.provider }) : item);
			this.replaceResolvedItems(items, resolvedItems);
			widget.updateResolvedCodeLensItems(resolvedItems);
			this.updateCache();
		} finally {
			if (this.resolvingWidgets.get(widget) === request) this.resolvingWidgets.delete(widget);
		}
	}

	private replaceResolvedItems(previousItems: readonly CodeLensItem[], resolvedItems: readonly CodeLensItem[]): void {
		const replacements = new Map(previousItems.map((item, index) => [item, resolvedItems[index]!] as const));
		this.currentModel = new CodeLensModel(this.currentModel.lenses.map(item => replacements.get(item) ?? item));
	}

	private updateCache(): void {
		if (this.resource) codeLensCache.put(this.resource, this.viewport.textModel.lineCount, this.currentModel);
	}

	private executeCommand(command: LanguageCodeLensCommand): void {
		try {
			const result = this.onExecuteCommand!(command.id, command.arguments);
			if (result && typeof (result as { readonly then?: unknown }).then === 'function') {
				void Promise.resolve(result as PromiseLike<void>).catch(this.onError);
			}
		} catch (error) {
			this.onError(error);
		}
	}
}

function groupCodeLensItems(items: readonly CodeLensItem[]): ReadonlyMap<number, readonly CodeLensItem[]> {
	const groups = new Map<number, CodeLensItem[]>();
	for (const item of items) {
		const lineIndex = item.symbol.range.start.lineIndex;
		const group = groups.get(lineIndex);
		if (group) group.push(item);
		else groups.set(lineIndex, [item]);
	}
	return groups;
}

registerEditorContribution({
	id: CodeLensContribution.ID,
	install: context => {
		if (context.kind !== 'text' || context.options.codeLens === false || context.model.largeFile.tooLargeForTokenization) return;
		context.register(new CodeLensContribution(
			context.viewport,
			context.languageFeaturesService.codeLensProvider,
			context.languageId,
			context.options.input.resource,
			context.options.onExecuteEditorCommand,
			context.onLanguageError,
		));
	},
});
