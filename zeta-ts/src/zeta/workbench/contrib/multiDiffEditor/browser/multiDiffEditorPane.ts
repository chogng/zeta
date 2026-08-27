import './media/multiDiffEditorPane.css';
import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import { h } from '../../../../base/browser/dom.js';
import { type IDimension } from '../../../../base/browser/geometry.js';
import { throwIfCancelled } from '../../../../base/common/cancellation.js';
import { Disposable, MutableDisposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { assertDefined } from '../../../../base/common/types.js';
import { MultiDiffEditorWidget, type MultiDiffEditorItem, type MultiDiffEditorLocation } from '../../../../editor/browser/widget/multiDiffEditor/multiDiffEditorWidget.js';
import { DiffModel } from '../../../../editor/common/diff/diffModel.js';
import { type IDiffComputationService } from '../../../../editor/common/diff/diffComputationService.js';
import { type ITextModelService, type TextModelReference } from '../../../../editor/common/services/textModelService.js';
import { MenuWorkbenchToolBar } from '../../../../platform/actions/browser/toolbar.js';
import { MenuId } from '../../../../platform/actions/common/actions.js';
import type { IMenuService } from '../../../../platform/actions/common/menuService.js';
import type { IContextKeyService } from '../../../../platform/contextkey/common/contextkey.js';
import { type EditorInput } from '../../../browser/parts/editor/editorInput.js';
import { type IEditorPane } from '../../../browser/parts/editor/editorPane.js';
import { EditorPaneVisibility } from '../../../browser/parts/editor/editorPane.js';
import { isMultiDiffEditorInput, MULTI_DIFF_EDITOR_ID, multiDiffEditorItemKey, type MultiDiffEditorInputItem } from './multiDiffEditorInput.js';

export interface MultiDiffEditorPaneOptions {
	readonly modelService: ITextModelService;
	readonly createComputationService: () => IDiffComputationService;
	readonly lineHeight?: number;
	readonly fontFamily?: string;
	readonly fontSize?: number;
	readonly fontLigatures?: boolean;
	readonly showLineNumbers?: boolean;
	readonly showInlineChanges?: boolean;
	readonly loopChanges?: boolean;
	readonly fileActions?: {
		readonly menuService: IMenuService;
		readonly contextMenuProvider: IContextMenuProvider;
		readonly contextKeyService?: IContextKeyService;
	};
}

interface ResolvedMultiDiffItem {
	readonly input: MultiDiffEditorInputItem;
	readonly original: TextModelReference;
	readonly modified: TextModelReference;
}

/** Workbench pane that resolves multi-diff inputs and hosts the generic editor widget. */
export class MultiDiffEditorPane extends Disposable implements IEditorPane {
	public readonly id = MULTI_DIFF_EDITOR_ID;
	private readonly session = this._register(new MutableDisposable<MultiDiffEditorPaneSession>());
	private editorContainerDomNode: HTMLDivElement | undefined;
	private dimension: IDimension = { width: 0, height: 0 };

	constructor(private readonly options: MultiDiffEditorPaneOptions) {
		super();
		if (!options || typeof options !== 'object' || typeof options.createComputationService !== 'function') {
			this.dispose();
			throw new TypeError('Multi-diff editor pane requires the Rust diff computation service');
		}
		if (!options.modelService || typeof options.modelService.acquire !== 'function') {
			this.dispose();
			throw new TypeError('Multi-diff editor pane requires a text model service');
		}
	}

	public create(parent: HTMLElement): void {
		if (this.editorContainerDomNode) throw new ReferenceError('MultiDiffEditorPane has already been created');
		const editorContainerDomNode = h(parent.ownerDocument, 'div');
		editorContainerDomNode.className = 'stanza-multi-diff-editor-pane';
		parent.append(editorContainerDomNode);
		this.editorContainerDomNode = editorContainerDomNode;
		this._register(toDisposable(() => {
			editorContainerDomNode.remove();
			this.editorContainerDomNode = undefined;
		}));
	}

	public async setInput(input: EditorInput, signal: AbortSignal): Promise<void> {
		if (!isMultiDiffEditorInput(input)) throw new TypeError('Multi-diff editor pane requires a multi-diff editor input');
		const container = this.requireContainer();
		throwIfCancelled(signal, 'Multi-diff editor input loading was cancelled');
		const resolved: ResolvedMultiDiffItem[] = [];
		let referencesOwnedBySession = false;
		let next: MultiDiffEditorPaneSession | undefined;
		try {
			for (const item of input.items) {
				const original = await this.options.modelService.acquire(item.original, signal);
				let modified: TextModelReference | undefined;
				try {
					throwIfCancelled(signal, 'Multi-diff editor input loading was cancelled');
					modified = await this.options.modelService.acquire(item.modified, signal);
					resolved.push({ input: item, original, modified });
				} catch (error) {
					modified?.dispose();
					original.dispose();
					throw error;
				}
			}
			throwIfCancelled(signal, 'Multi-diff editor input loading was cancelled');
			referencesOwnedBySession = true;
			next = new MultiDiffEditorPaneSession(container, resolved, input.label ?? 'Changes', this.options);
			throwIfCancelled(signal, 'Multi-diff editor input loading was cancelled');
		} catch (error) {
			next?.dispose();
			if (!referencesOwnedBySession) disposeResolvedItems(resolved);
			throw error;
		}
		this.session.value = next;
		next.layout(this.dimension);
	}

	public clearInput(): void {
		this.session.clear();
	}

	public layout(dimension: IDimension): void {
		this.dimension = { width: Math.max(0, dimension.width), height: Math.max(0, dimension.height) };
		this.session.value?.layout(this.dimension);
	}

	public setVisible(visibility: EditorPaneVisibility): void {
		if (!this.editorContainerDomNode) return;
		this.editorContainerDomNode.hidden = visibility === EditorPaneVisibility.Hidden;
		if (visibility === EditorPaneVisibility.Visible) this.session.value?.layout(this.dimension);
	}

	public focus(): void {
		this.session.value?.focus();
	}

	public nextChange(): MultiDiffEditorLocation | undefined {
		return this.session.value?.editor.nextChange();
	}

	public previousChange(): MultiDiffEditorLocation | undefined {
		return this.session.value?.editor.previousChange();
	}

	public collapseAll(): void {
		this.session.value?.editor.collapseAll();
	}

	public expandAll(): void {
		this.session.value?.editor.expandAll();
	}

	private requireContainer(): HTMLDivElement {
		assertDefined(this.editorContainerDomNode, new ReferenceError('Multi-diff editor pane has not been created'));
		return this.editorContainerDomNode;
	}
}

class MultiDiffEditorPaneSession extends Disposable {
	public readonly editor: MultiDiffEditorWidget;

	constructor(container: HTMLElement, resolved: readonly ResolvedMultiDiffItem[], label: string, options: MultiDiffEditorPaneOptions) {
		super();
		try {
			for (const item of resolved) {
				this._register(item.original);
				this._register(item.modified);
			}
			const computationService = options.createComputationService();
			if (!computationService || typeof computationService.compute !== 'function') {
				throw new TypeError('Multi-diff editor pane factory returned an invalid Rust diff computation service');
			}
			this._register(computationService);
			const items: MultiDiffEditorItem[] = resolved.map((item) => ({
				id: multiDiffEditorItemKey(item.input),
				label: item.input.label,
				originalLabel: item.input.original.label,
				modifiedLabel: item.input.modified.label,
				model: this._register(new DiffModel({
					original: item.original.model,
					modified: item.modified.model,
					computationService,
				})),
			}));
			const inputsById = new Map(resolved.map((item) => [multiDiffEditorItemKey(item.input), item.input]));
			const fileActions = options.fileActions;
			this.editor = this._register(new MultiDiffEditorWidget({
				container,
				items,
				lineHeight: options.lineHeight,
				fontFamily: options.fontFamily,
				fontSize: options.fontSize,
				fontLigatures: options.fontLigatures,
				showLineNumbers: options.showLineNumbers,
				showInlineChanges: options.showInlineChanges,
				loopChanges: options.loopChanges,
				ariaLabel: `${label}, ${items.length} files`,
				...(fileActions ? {
					createItemActions: (container: HTMLElement, item: MultiDiffEditorItem) => {
						const input = inputsById.get(item.id);
						if (!input) throw new RangeError(`Unknown multi-diff item '${item.id}'`);
						return new MenuWorkbenchToolBar(
							container,
							fileActions.menuService,
							fileActions.contextMenuProvider,
							MenuId.MultiDiffEditorFileToolbar,
							{
								ariaLabel: `${input.label} actions`,
								contextKeyService: fileActions.contextKeyService,
								menuOptions: { arg: input.goToFile ?? input.modified },
								presentation: 'inherit-foreground',
							},
						);
					},
				} : {}),
			}));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	public layout(dimension: IDimension): void {
		this.editor.layout(dimension);
	}

	public focus(): void {
		this.editor.domNode.focus({ preventScroll: true });
	}
}

function disposeResolvedItems(items: readonly ResolvedMultiDiffItem[]): void {
	for (let index = items.length - 1; index >= 0; index -= 1) {
		items[index]!.modified.dispose();
		items[index]!.original.dispose();
	}
}
