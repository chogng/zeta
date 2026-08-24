import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import { addDisposableListener, h } from '../../../../base/browser/dom.js';
import { disposableWindowTimeout } from '../../../../base/browser/scheduler.js';
import { Button } from '../../../../base/browser/ui/button/button.js';
import { Checkbox } from '../../../../base/browser/ui/toggle/toggle.js';
import { DisposableOwner, DisposableSlot, ResettableDisposableGroup, type IDisposable } from '../../../../base/common/lifecycle.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import { SemanticCodeIndexSettingId, type CodeIndexStatus, type ICodeIndexService, type SemanticCodeIndexSelection } from '../../../../platform/codeIndex/common/codeIndexService.js';
import type { IDialogService } from '../../../../platform/dialogs/common/dialogs.js';
import { ToolSearchSettingId, type IToolSearchService, type ToolSearchEmbeddingStatus } from '../../../../platform/toolSearch/common/toolSearchService.js';
import { SettingsItemActions } from './settingsItemActions.js';

interface IndexingSettingsPaneOptions {
	readonly clipboardService: IClipboardService;
	readonly codeIndexService: ICodeIndexService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly dialogService: IDialogService;
	readonly toolSearchService: IToolSearchService;
}

/** Owns Tool Search and semantic code-index settings independently of the Settings shell. */
export class IndexingSettingsPane extends DisposableOwner {
	public readonly element: HTMLDivElement;
	private readonly renderBindings = this.own(new ResettableDisposableGroup());
	private active = true;
	private renderRevision = 0;

	constructor(container: HTMLElement, private readonly options: IndexingSettingsPaneOptions) {
		super();
		this.element = h(container.ownerDocument, 'div');
		this.element.className = 'zeta-indexing-settings';
		container.append(this.element);
		this.defer(() => {
			this.active = false;
			this.renderRevision += 1;
			this.element.remove();
		});
	}

	public activate(): void {
		void this.refresh();
	}

	private async refresh(): Promise<void> {
		const revision = ++this.renderRevision;
		this.renderBindings.clear();
		const document = this.element.ownerDocument;
		const loading = h(document, 'p');
		loading.className = 'zeta-settings-message';
		loading.textContent = 'Loading search settings…';
		this.element.replaceChildren(loading);
		const loaded = await Promise.all([
			this.options.codeIndexService.readConfig(),
			this.options.toolSearchService.readConfig(),
		]).catch((error: unknown) => {
			loading.textContent = error instanceof Error ? `Unable to load indexing settings: ${error.message}` : 'Unable to load indexing settings.';
			return undefined;
		});
		if (!loaded || !this.active || revision !== this.renderRevision) return;
		const [codeConfig, toolConfig] = loaded;

		const toolItem = h(document, 'div');
		toolItem.className = 'zeta-indexing-setting-item';
		const toolGroup = h(document, 'fieldset');
		toolGroup.className = 'zeta-indexing-setting';
		const toolLegend = h(document, 'legend');
		toolLegend.textContent = 'Agent tool search';
		const toolHint = h(document, 'p');
		toolHint.className = 'zeta-theme-setting-hint';
		toolHint.textContent = 'Lexical search keeps tool metadata local. Hybrid search sends tool names, descriptions, schemas, and the query to the selected embedding model, then merges that ranking with BM25.';
		const toolEnabled = this.renderBindings.add(new Checkbox(toolGroup, {
			label: 'Use hybrid embedding search',
			checked: toolConfig.mode === 'hybridEmbedding',
		}));
		toolEnabled.element.classList.add('zeta-indexing-toggle');
		const toolEmbedding = h(document, 'input');
		toolEmbedding.className = 'zeta-settings-text-input';
		toolEmbedding.placeholder = 'provider/model (for example ollama/nomic-embed-text)';
		toolEmbedding.setAttribute('aria-label', 'Tool Search embedding model');
		toolEmbedding.value = toolConfig.embeddingModel ? formatModel(toolConfig.embeddingModel) : '';
		toolEmbedding.disabled = !toolEnabled.checked;
		const toolStatus = h(document, 'p');
		toolStatus.className = 'zeta-theme-setting-status';
		toolStatus.setAttribute('role', 'status');
		toolStatus.textContent = toolSearchStatusMessage(toolConfig.embeddingStatus);
		const toolActions = h(document, 'div');
		toolActions.className = 'zeta-theme-json-actions';
		const toolSave = this.renderBindings.add(new Button(toolActions, {
			label: 'Save tool search',
			presentation: 'primary',
		}));
		this.renderBindings.add(toolEnabled.onDidChange(() => {
			toolEmbedding.disabled = !toolEnabled.checked;
		}));
		this.renderBindings.add(toolSave.onDidClick(() => {
			let embeddingModel = toolConfig.embeddingModel;
			try {
				if (toolEnabled.checked) embeddingModel = parseModel(toolEmbedding.value, 'Tool Search embedding model');
			} catch (error) {
				toolStatus.textContent = error instanceof Error ? error.message : 'Invalid Tool Search model.';
				return;
			}
			toolGroup.disabled = true;
			void this.options.toolSearchService.configure({
				mode: toolEnabled.checked ? 'hybridEmbedding' : 'lexical',
				embeddingModel: toolEnabled.checked ? embeddingModel : undefined,
			}, toolConfig.revision).then(() => this.refresh()).catch((error: unknown) => {
				toolStatus.textContent = error instanceof Error ? `Unable to save: ${error.message}` : 'Unable to save.';
			}).finally(() => {
				toolGroup.disabled = false;
			});
		}));
		toolGroup.append(toolLegend, toolHint, toolEnabled.element, toolEmbedding, toolStatus, toolActions);
		toolItem.append(toolGroup);
		this.renderBindings.add(new SettingsItemActions(toolItem, {
			label: 'Agent tool search',
			reference: {
				id: ToolSearchSettingId,
				isDefault: () => toolConfig.mode === 'lexical' && toolConfig.embeddingModel === undefined,
				reset: () => this.options.toolSearchService.configure({ mode: 'lexical' }, toolConfig.revision).then(() => this.refresh()),
			},
			contextMenuProvider: this.options.contextMenuProvider,
			clipboardService: this.options.clipboardService,
			onError: error => {
				toolStatus.textContent = error instanceof Error ? error.message : 'Unable to run the setting action.';
			},
		}));

		const semanticItem = h(document, 'div');
		semanticItem.className = 'zeta-indexing-setting-item';
		const group = h(document, 'fieldset');
		group.className = 'zeta-indexing-setting';
		const legend = h(document, 'legend');
		legend.textContent = 'Semantic code search';
		const hint = h(document, 'p');
		hint.className = 'zeta-theme-setting-hint';
		hint.textContent = 'Zeta keeps chunking, vectors, recall, fusion, and Agent results local. When enabled and authorized, bounded code chunks and search queries are sent to the selected model endpoint.';
		const providerHeading = h(document, 'h4');
		providerHeading.textContent = 'Model endpoint';
		const provider = h(document, 'input');
		provider.className = 'zeta-settings-text-input';
		provider.placeholder = 'ollama or openai-compatible';
		provider.setAttribute('aria-label', 'Semantic model provider');
		const endpoint = h(document, 'input');
		endpoint.className = 'zeta-settings-text-input';
		endpoint.placeholder = 'http://localhost:11434/v1';
		endpoint.setAttribute('aria-label', 'Semantic model endpoint URL');
		const configuredProvider = codeConfig.semanticCodeIndex.selection.type === 'remote'
			? codeConfig.semanticCodeIndex.selection.models.embeddingModel.provider
			: 'ollama';
		provider.value = configuredProvider;
		endpoint.value = codeConfig.providers[configuredProvider]?.baseUrl ?? (configuredProvider === 'ollama' ? 'http://localhost:11434/v1' : '');
		const providerActions = h(document, 'div');
		providerActions.className = 'zeta-theme-json-actions';
		const providerSave = this.renderBindings.add(new Button(providerActions, {
			label: 'Save endpoint',
			presentation: 'primary',
		}));
		const enabled = this.renderBindings.add(new Checkbox(group, {
			label: 'Use an embedding/rerank model endpoint',
			checked: codeConfig.semanticCodeIndex.selection.type === 'remote',
		}));
		enabled.element.classList.add('zeta-indexing-toggle');
		const embedding = h(document, 'input');
		embedding.className = 'zeta-settings-text-input';
		embedding.placeholder = 'provider/model (for example ollama/nomic-embed-text)';
		embedding.setAttribute('aria-label', 'Embedding model');
		const rerank = h(document, 'input');
		rerank.className = 'zeta-settings-text-input';
		rerank.placeholder = 'Optional openai-compatible/model reranker';
		rerank.setAttribute('aria-label', 'Rerank model');
		if (codeConfig.semanticCodeIndex.selection.type === 'remote') {
			embedding.value = formatModel(codeConfig.semanticCodeIndex.selection.models.embeddingModel);
			rerank.value = codeConfig.semanticCodeIndex.selection.models.rerankModel ? formatModel(codeConfig.semanticCodeIndex.selection.models.rerankModel) : '';
		}
		embedding.disabled = !enabled.checked;
		rerank.disabled = !enabled.checked;
		const automaticContext = this.renderBindings.add(new Checkbox(group, {
			label: 'Automatically add verified code excerpts to the first Agent request',
			checked: codeConfig.semanticCodeIndex.automaticContext === 'firstInvocation',
			disabled: !enabled.checked,
		}));
		automaticContext.element.classList.add('zeta-indexing-toggle');
		const status = h(document, 'p');
		status.className = 'zeta-theme-setting-status';
		status.setAttribute('role', 'status');
		status.textContent = codeConfig.semanticCodeIndex.activeWorkspaceAuthorized
			? 'The active workspace is authorized for this exact model selection.'
			: 'The active workspace is not authorized; no source text will be sent.';
		const progress = h(document, 'p');
		progress.className = 'zeta-theme-setting-status';
		progress.setAttribute('role', 'status');
		progress.textContent = 'Semantic index status is loading…';
		const jobActions = h(document, 'div');
		jobActions.className = 'zeta-theme-json-actions';
		const cancelJob = this.renderBindings.add(new Button(jobActions, {
			label: 'Cancel indexing',
			presentation: 'secondary',
		}));
		const retryJob = this.renderBindings.add(new Button(jobActions, {
			label: 'Retry indexing',
			presentation: 'secondary',
		}));
		const updateJobStatus = (indexStatus: CodeIndexStatus): void => {
			progress.textContent = semanticIndexStatusMessage(indexStatus);
			cancelJob.enabled = indexStatus.semantic.state === 'syncing';
			retryJob.enabled = codeConfig.semanticCodeIndex.activeWorkspaceAuthorized && indexStatus.semantic.state !== 'syncing';
		};
		let polling = true;
		const timer = this.renderBindings.add(new DisposableSlot<IDisposable>());
		const targetWindow = document.defaultView;
		this.renderBindings.defer(() => polling = false);
		const poll = (): void => {
			void this.options.codeIndexService.status().then(indexStatus => {
				if (!polling) return;
				updateJobStatus(indexStatus);
			}).catch(() => {
				if (polling) progress.textContent = 'Unable to read semantic index progress.';
			}).finally(() => {
				if (polling && targetWindow) timer.replace(disposableWindowTimeout(targetWindow, poll, 750));
			});
		};
		this.renderBindings.add(cancelJob.onDidClick(() => {
			cancelJob.enabled = false;
			void this.options.codeIndexService.cancel().then(updateJobStatus).catch(() => {
				progress.textContent = 'Unable to cancel semantic indexing.';
			});
		}));
		this.renderBindings.add(retryJob.onDidClick(() => {
			retryJob.enabled = false;
			void this.options.codeIndexService.retry().then(updateJobStatus).catch(() => {
				progress.textContent = 'Unable to retry semantic indexing.';
			});
		}));
		poll();
		this.renderBindings.add(addDisposableListener(provider, 'change', () => {
			const configured = codeConfig.providers[provider.value.trim()];
			endpoint.value = configured?.baseUrl ?? (provider.value.trim() === 'ollama' ? 'http://localhost:11434/v1' : '');
		}));
		this.renderBindings.add(providerSave.onDidClick(() => {
			const providerId = provider.value.trim();
			const baseUrl = endpoint.value.trim();
			if (!providerId) {
				status.textContent = 'Model provider is required.';
				return;
			}
			if (providerId === 'openai') {
				status.textContent = 'OpenAI API-key storage is not available in this settings page yet. Use Ollama or an unauthenticated OpenAI-compatible endpoint.';
				return;
			}
			if (!baseUrl) {
				status.textContent = 'Model endpoint URL is required.';
				return;
			}
			group.disabled = true;
			void this.options.codeIndexService.configureProvider({
				provider: providerId,
				baseUrl,
				maxOutputTokens: null,
				modelContext: {},
			}, codeConfig.revision).then(() => this.refresh()).catch((error: unknown) => {
				status.textContent = error instanceof Error ? `Unable to save endpoint: ${error.message}` : 'Unable to save endpoint.';
			}).finally(() => {
				group.disabled = false;
			});
		}));
		const actions = h(document, 'div');
		actions.className = 'zeta-theme-json-actions';
		const save = this.renderBindings.add(new Button(actions, {
			label: 'Save model selection',
			presentation: 'primary',
		}));
		const consent = this.renderBindings.add(new Button(actions, {
			label: codeConfig.semanticCodeIndex.activeWorkspaceAuthorized ? 'Revoke workspace access' : 'Authorize active workspace',
			presentation: codeConfig.semanticCodeIndex.activeWorkspaceAuthorized ? 'danger' : 'secondary',
			enabled: enabled.checked,
		}));
		this.renderBindings.add(enabled.onDidChange(() => {
			embedding.disabled = !enabled.checked;
			rerank.disabled = !enabled.checked;
			automaticContext.enabled = enabled.checked;
			consent.enabled = enabled.checked;
		}));
		this.renderBindings.add(save.onDidClick(() => {
			let selection: SemanticCodeIndexSelection = { type: 'disabled' };
			try {
				if (enabled.checked) {
					selection = {
						type: 'remote',
						models: {
							embeddingModel: parseModel(embedding.value, 'Embedding model'),
							rerankModel: rerank.value.trim() ? parseModel(rerank.value, 'Rerank model') : null,
						},
					};
				}
			} catch (error) {
				status.textContent = error instanceof Error ? error.message : 'Invalid model selection.';
				return;
			}
			group.disabled = true;
			void this.options.codeIndexService.configure(selection, automaticContext.checked ? 'firstInvocation' : 'off', codeConfig.revision).then(() => this.refresh()).catch((error: unknown) => {
				status.textContent = error instanceof Error ? `Unable to save: ${error.message}` : 'Unable to save.';
			}).finally(() => {
				group.disabled = false;
			});
		}));
		this.renderBindings.add(consent.onDidClick(() => {
			if (!codeConfig.semanticCodeIndex.activeWorkspaceAuthorized) {
				void this.confirmSemanticCodeIndexAuthorization(group, status, codeConfig.revision);
				return;
			}
			group.disabled = true;
			void this.options.codeIndexService.revoke(codeConfig.revision).then(() => this.refresh()).catch((error: unknown) => {
				status.textContent = error instanceof Error ? `Unable to update authorization: ${error.message}` : 'Unable to update authorization.';
			}).finally(() => {
				group.disabled = false;
			});
		}));
		group.append(legend, hint, providerHeading, provider, endpoint, providerActions, enabled.element, embedding, rerank, automaticContext.element, status, progress, jobActions, actions);
		semanticItem.append(group);
		this.renderBindings.add(new SettingsItemActions(semanticItem, {
			label: 'Semantic code search',
			reference: {
				id: SemanticCodeIndexSettingId,
				isDefault: () => codeConfig.semanticCodeIndex.selection.type === 'disabled'
					&& codeConfig.semanticCodeIndex.automaticContext === 'off'
					&& !codeConfig.semanticCodeIndex.activeWorkspaceAuthorized,
				reset: async () => {
					const result = await this.options.codeIndexService.configure({ type: 'disabled' }, 'off', codeConfig.revision);
					if (codeConfig.semanticCodeIndex.activeWorkspaceAuthorized) await this.options.codeIndexService.revoke(result.revision);
					await this.refresh();
				},
			},
			contextMenuProvider: this.options.contextMenuProvider,
			clipboardService: this.options.clipboardService,
			onError: error => {
				status.textContent = error instanceof Error ? error.message : 'Unable to run the setting action.';
			},
		}));
		this.element.replaceChildren(toolItem, semanticItem);
	}

	private async confirmSemanticCodeIndexAuthorization(group: HTMLFieldSetElement, status: HTMLParagraphElement, revision: number): Promise<void> {
		const confirmed = await this.options.dialogService.confirm({
			title: 'Authorize semantic code search?',
			message: 'Allow the selected model endpoint to process source-derived text from this workspace?',
			detail: 'Zeta sends bounded code chunks while building embeddings, search queries, and—when configured—recalled candidate text for reranking. Chunking, vector storage, recall, fusion, and final Agent results stay local. This permission is tied to this workspace, model selection, and endpoint, and can be revoked here.',
			primaryButton: 'Authorize workspace',
			cancelButton: 'Cancel',
		});
		if (!confirmed) return;
		group.disabled = true;
		try {
			await this.options.codeIndexService.authorize(revision);
			await this.refresh();
		} catch (error) {
			status.textContent = error instanceof Error ? `Unable to update authorization: ${error.message}` : 'Unable to update authorization.';
		} finally {
			group.disabled = false;
		}
	}
}

function toolSearchStatusMessage(status: ToolSearchEmbeddingStatus): string {
	switch (status.type) {
		case 'disabled':
			return 'Local BM25 and Regex search are active; no tool metadata is sent to a model.';
		case 'ready':
			return `Embedding search is ready with ${formatModel(status.model)}.`;
		case 'unavailable':
			return `Embedding search is unavailable: ${status.reason}`;
	}
}

function semanticIndexStatusMessage(status: CodeIndexStatus): string {
	const semantic = status.semantic;
	switch (semantic.state) {
		case 'unavailable':
			return 'Semantic indexing is unavailable until a model is configured and authorized.';
		case 'idle':
			return 'Semantic indexing has not started.';
		case 'syncing': {
			const total = semantic.totalChunkCount;
			const progress = total > 0 ? ` ${semantic.processedChunkCount}/${total} chunks` : '';
			const retries = semantic.retryCount > 0 ? ` · ${semantic.retryCount} retries` : '';
			return `Semantic indexing: ${semantic.phase ?? 'preparing'}${progress}${retries}.`;
		}
		case 'ready':
			return `Semantic index is ready for generation ${semantic.publishedGeneration ?? status.generation}.`;
		case 'stale':
			return 'Semantic index is stale and waiting to catch up.';
		case 'cancelled':
			return `Semantic indexing was cancelled after ${semantic.processedChunkCount} chunks; completed batches are cached.`;
		case 'failed':
			return `Semantic indexing failed (${semantic.lastErrorCode ?? 'unknown'}); completed batches are cached for retry.`;
	}
}

function formatModel(model: { readonly provider: string; readonly model: string }): string {
	return `${model.provider}/${model.model}`;
}

function parseModel(value: string, label: string): { provider: string; model: string } {
	const separator = value.indexOf('/');
	const provider = separator < 0 ? '' : value.slice(0, separator).trim();
	const model = separator < 0 ? '' : value.slice(separator + 1).trim();
	if (!provider || !model) throw new Error(`${label} must use provider/model.`);
	return { provider, model };
}
