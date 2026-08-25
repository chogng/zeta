import type { Event } from '../../base/common/event.js';
import {
	DisposableOwner,
	DisposableSlot,
	type IDisposable,
	toDisposable,
} from '../../base/common/lifecycle.js';
import type {
	IContextKey,
	IContextKeyService,
} from '../../platform/contextkey/common/contextkey.js';
import { isTextResourceLanguageInput, resolveTextResourceLanguageId, type TextResourceLanguageResolver } from '../../platform/language/common/textResourceLanguage.js';
import type {
	IWorkspaceContextService,
} from '../../platform/workspace/common/workspace.js';
import {
	ActiveAgentSidebarContext,
	ActiveAuxiliaryContext,
	ActiveEditorAvailableEditorIdsContext,
	ActiveEditorCanRevertContext,
	ActiveEditorContext,
	ActiveEditorDirtyContext,
	ActiveEditorFirstInGroupContext,
	ActiveEditorGroupEmptyContext,
	ActiveEditorGroupIndexContext,
	ActiveEditorGroupLastContext,
	ActiveEditorLastInGroupContext,
	ActiveEditorPinnedContext,
	ActiveEditorReadonlyContext,
	ActivePanelContext,
	ActiveViewletContext,
	AgentSidebarVisibleContext,
	AuxiliaryBarVisibleContext,
	DirtyWorkingCopiesContext,
	EditorAreaVisibleContext,
	EditorGroupEditorsCountContext,
	EditorPartModalVisibleContext,
	EditorsVisibleContext,
	MultipleEditorGroupsContext,
	PanelVisibleContext,
	ResourceContext,
	ResourceDirnameContext,
	ResourceExtensionContext,
	ResourceFilenameContext,
	ResourceLanguageIdContext,
	ResourcePathContext,
	ResourceSchemeContext,
	ResourceSetContext,
	SideBarVisibleContext,
	WorkbenchStateContext,
	WorkspaceFolderCountContext,
	workbenchStateToContextValue,
} from '../common/contextkeys.js';
import type { IWorkingCopy, IWorkingCopyService } from '../services/workingCopy/common/workingCopyService.js';
import type { IEditorPart } from './parts/editor/editorPart.js';
import type { EditorPaneRegistry } from './parts/editor/editorRegistry.js';
import type { IWorkbenchLayoutService, WorkbenchPartId } from '../services/layout/browser/layoutService.js';

/** Keeps window-wide Workbench context keys synchronized with the workspace. */
export function bindWorkbenchContextKeys(
	contextKeyService: IContextKeyService,
	workspaceContextService: IWorkspaceContextService,
	workingCopyService: IWorkingCopyService,
): IDisposable {
	const { workbenchState, workspaceFolderCount, dirtyWorkingCopies } = bufferContextKeyChanges(contextKeyService, () => {
		const workbenchState = WorkbenchStateContext.bindTo(contextKeyService);
		const workspaceFolderCount = WorkspaceFolderCountContext.bindTo(contextKeyService);
		const dirtyWorkingCopies = DirtyWorkingCopiesContext.bindTo(contextKeyService);
		workbenchState.set(workbenchStateToContextValue(workspaceContextService.getWorkbenchState()));
		workspaceFolderCount.set(workspaceContextService.getWorkspace().folders.length);
		dirtyWorkingCopies.set(workingCopyService.hasDirtyWorkingCopies);
		return { workbenchState, workspaceFolderCount, dirtyWorkingCopies };
	});

	const updateWorkspaceKeys = (): void => {
		workbenchState.set(workbenchStateToContextValue(
			workspaceContextService.getWorkbenchState(),
		));
		workspaceFolderCount.set(
			workspaceContextService.getWorkspace().folders.length,
		);
	};
	const updateDirtyWorkingCopies = (): void => {
		dirtyWorkingCopies.set(workingCopyService.hasDirtyWorkingCopies);
	};
	const workspaceSubscription =
		workspaceContextService.onDidChangeWorkspace(updateWorkspaceKeys);
	const dirtySubscription = workingCopyService.onDidChangeDirty(updateDirtyWorkingCopies);

	return toDisposable(() => {
		dirtySubscription.dispose();
		workspaceSubscription.dispose();
		contextKeyService.bufferChangeEvents(() => {
			dirtyWorkingCopies.reset();
			workspaceFolderCount.reset();
			workbenchState.reset();
		});
	});
}

/** Keeps layout-owned visibility keys synchronized with the browser shell. */
export function bindWorkbenchPartVisibilityContextKeys(
	contextKeyService: IContextKeyService,
	layoutService: IWorkbenchLayoutService,
): IDisposable {
	const visibilityKeys = bufferContextKeyChanges(contextKeyService, () => {
		const visibilityKeys = new Map<WorkbenchPartId, IContextKey<boolean>>([
			['sidebar', SideBarVisibleContext.bindTo(contextKeyService)],
			['auxiliarybar', AuxiliaryBarVisibleContext.bindTo(contextKeyService)],
			['agentSidebar', AgentSidebarVisibleContext.bindTo(contextKeyService)],
			['panel', PanelVisibleContext.bindTo(contextKeyService)],
			['editor', EditorAreaVisibleContext.bindTo(contextKeyService)],
		]);
		for (const [partId, key] of visibilityKeys) key.set(layoutService.isPartVisible(partId));
		return visibilityKeys;
	});
	const subscription = layoutService.onDidChangePartVisibility(({ partId, visible }) => {
		visibilityKeys.get(partId)?.set(visible);
	});
	return toDisposable(() => {
		subscription.dispose();
		contextKeyService.bufferChangeEvents(() => {
			for (const key of visibilityKeys.values()) key.reset();
		});
	});
}

export interface WorkbenchCompositeContextSources {
	readonly sidebar: WorkbenchCompositeContextSource;
	readonly auxiliarybar: WorkbenchCompositeContextSource;
	readonly agentSidebar: WorkbenchCompositeContextSource;
	readonly panel: WorkbenchCompositeContextSource;
}

export interface WorkbenchCompositeContextSource {
	readonly activeCompositeId: string | undefined;
	readonly onDidChangeActiveComposite: Event<string>;
}

/** Projects the active container retained by each Pane Composite Part. */
export function bindWorkbenchActiveCompositeContextKeys(
	contextKeyService: IContextKeyService,
	sources: WorkbenchCompositeContextSources,
): IDisposable {
	const bindings = bufferContextKeyChanges(contextKeyService, () => {
		const bindings = [
			{ source: sources.sidebar, key: ActiveViewletContext.bindTo(contextKeyService) },
			{ source: sources.auxiliarybar, key: ActiveAuxiliaryContext.bindTo(contextKeyService) },
			{ source: sources.agentSidebar, key: ActiveAgentSidebarContext.bindTo(contextKeyService) },
			{ source: sources.panel, key: ActivePanelContext.bindTo(contextKeyService) },
		];
		for (const { source, key } of bindings) key.set(source.activeCompositeId ?? '');
		return bindings;
	});
	const subscriptions = bindings.map(({ source, key }) => source.onDidChangeActiveComposite(id => key.set(id)));
	return toDisposable(() => {
		for (const subscription of subscriptions) subscription.dispose();
		contextKeyService.bufferChangeEvents(() => {
			for (const { key } of bindings) key.reset();
		});
	});
}

/** Projects the canonical Editor Part state into root Workbench context keys. */
export function bindEditorContextKeys(
	contextKeyService: IContextKeyService,
	editorPart: IEditorPart,
	editorRegistry: EditorPaneRegistry,
	languageResolver?: TextResourceLanguageResolver,
): IDisposable {
	return new EditorContextKeyController(contextKeyService, editorPart, editorRegistry, languageResolver);
}

type EditorContextKeyBindings = ReturnType<typeof createEditorContextKeyBindings>;

class EditorContextKeyController extends DisposableOwner {
	private readonly keys: EditorContextKeyBindings;
	private readonly workingCopyListener = this.own(new DisposableSlot<IDisposable>());
	private activeWorkingCopy: IWorkingCopy | undefined;

	constructor(
		private readonly contextKeyService: IContextKeyService,
		private readonly editorPart: IEditorPart,
		private readonly editorRegistry: EditorPaneRegistry,
		private readonly languageResolver: TextResourceLanguageResolver | undefined,
	) {
		super();
		this.keys = bufferContextKeyChanges(contextKeyService, () => {
			const keys = createEditorContextKeyBindings(contextKeyService);
			this.update(keys);
			return keys;
		});
		this.own(this.editorPart.onDidChangeEditors(() => this.update()));
		this.defer(() => this.reset());
	}

	private update(keys: EditorContextKeyBindings = this.keys): void {
		const input = this.editorPart.activeInput;
		const pane = this.editorPart.activePane;
		const group = this.editorPart.activeGroup;
		const workingCopy = pane?.workingCopy;
		this.updateWorkingCopyListener(workingCopy, keys.activeEditorDirty);
		const activeIndex = input && !this.editorPart.isModalEditorVisible ? group.inputs.indexOf(input) : -1;
		const resource = input?.resource;
		const path = resource ? resourceContextPath(resource) : undefined;
		const filename = path ? resourceFilename(path) : undefined;
		this.contextKeyService.bufferChangeEvents(() => {
			keys.activeEditor.set(pane?.id ?? '');
			keys.activeEditorDirty.set(workingCopy?.isDirty ?? false);
			keys.activeEditorPinned.set(Boolean(input && (this.editorPart.isModalEditorVisible || !group.isPreview(input))));
			keys.activeEditorFirstInGroup.set(activeIndex === 0);
			keys.activeEditorLastInGroup.set(activeIndex >= 0 && activeIndex === group.inputs.length - 1);
			keys.activeEditorReadonly.set(input?.readOnly === true);
			keys.activeEditorCanRevert.set(workingCopy !== undefined);
			keys.activeEditorAvailableEditorIds.set(input ? this.editorRegistry.getEditors(input).map(editor => editor.id).join(',') : '');
			keys.editorGroupEditorsCount.set(group.inputs.length);
			keys.activeEditorGroupEmpty.set(input === undefined);
			const activeGroupIndex = this.editorPart.groups.indexOf(group);
			keys.activeEditorGroupIndex.set(activeGroupIndex >= 0 ? activeGroupIndex + 1 : 0);
			keys.activeEditorGroupLast.set(activeGroupIndex >= 0 && activeGroupIndex === this.editorPart.groups.length - 1);
			keys.multipleEditorGroups.set(this.editorPart.groups.length > 1);
			keys.editorsVisible.set(this.editorPart.isModalEditorVisible || this.editorPart.groups.some(candidate => candidate.inputs.length > 0));
			keys.editorPartModalVisible.set(this.editorPart.isModalEditorVisible);
			keys.resource.set(resource?.toString());
			keys.resourceScheme.set(resource?.scheme);
			keys.resourceFilename.set(filename);
			keys.resourceDirname.set(path ? resourceDirname(path) : undefined);
			keys.resourcePath.set(path);
			keys.resourceLanguageId.set(input ? resourceLanguageId(input, this.languageResolver) : undefined);
			keys.resourceExtension.set(filename ? resourceExtension(filename) : undefined);
			keys.resourceSet.set(resource !== undefined);
		});
	}

	private updateWorkingCopyListener(workingCopy: IWorkingCopy | undefined, activeEditorDirty: IContextKey<boolean>): void {
		if (workingCopy === this.activeWorkingCopy) return;
		this.activeWorkingCopy = workingCopy;
		this.workingCopyListener.replace(workingCopy?.onDidChangeDirty(() => {
			if (this.activeWorkingCopy === workingCopy) activeEditorDirty.set(workingCopy.isDirty);
		}));
	}

	private reset(): void {
		this.activeWorkingCopy = undefined;
		this.workingCopyListener.clear();
		this.contextKeyService.bufferChangeEvents(() => {
			for (const key of this.keys.all) key.reset();
		});
	}
}

function createEditorContextKeyBindings(contextKeyService: IContextKeyService) {
	const bindings = {
		activeEditor: ActiveEditorContext.bindTo(contextKeyService),
		activeEditorDirty: ActiveEditorDirtyContext.bindTo(contextKeyService),
		activeEditorPinned: ActiveEditorPinnedContext.bindTo(contextKeyService),
		activeEditorFirstInGroup: ActiveEditorFirstInGroupContext.bindTo(contextKeyService),
		activeEditorLastInGroup: ActiveEditorLastInGroupContext.bindTo(contextKeyService),
		activeEditorReadonly: ActiveEditorReadonlyContext.bindTo(contextKeyService),
		activeEditorCanRevert: ActiveEditorCanRevertContext.bindTo(contextKeyService),
		activeEditorAvailableEditorIds: ActiveEditorAvailableEditorIdsContext.bindTo(contextKeyService),
		editorGroupEditorsCount: EditorGroupEditorsCountContext.bindTo(contextKeyService),
		activeEditorGroupEmpty: ActiveEditorGroupEmptyContext.bindTo(contextKeyService),
		activeEditorGroupIndex: ActiveEditorGroupIndexContext.bindTo(contextKeyService),
		activeEditorGroupLast: ActiveEditorGroupLastContext.bindTo(contextKeyService),
		multipleEditorGroups: MultipleEditorGroupsContext.bindTo(contextKeyService),
		editorsVisible: EditorsVisibleContext.bindTo(contextKeyService),
		editorPartModalVisible: EditorPartModalVisibleContext.bindTo(contextKeyService),
		resource: ResourceContext.bindTo(contextKeyService),
		resourceScheme: ResourceSchemeContext.bindTo(contextKeyService),
		resourceFilename: ResourceFilenameContext.bindTo(contextKeyService),
		resourceDirname: ResourceDirnameContext.bindTo(contextKeyService),
		resourcePath: ResourcePathContext.bindTo(contextKeyService),
		resourceLanguageId: ResourceLanguageIdContext.bindTo(contextKeyService),
		resourceExtension: ResourceExtensionContext.bindTo(contextKeyService),
		resourceSet: ResourceSetContext.bindTo(contextKeyService),
	};
	return {
		...bindings,
		all: Object.values(bindings) as readonly Pick<IContextKey<never>, 'reset'>[],
	};
}

function resourceContextPath(resource: NonNullable<IEditorPart['activeInput']>['resource']): string {
	return resource.scheme === 'file' ? resource.fsPath : decodeURIComponent(resource.path);
}

function resourceFilename(path: string): string | undefined {
	const normalized = path.replace(/[\\/]+$/u, '');
	if (!normalized) return undefined;
	const separator = Math.max(normalized.lastIndexOf('/'), normalized.lastIndexOf('\\'));
	return normalized.slice(separator + 1) || undefined;
}

function resourceDirname(path: string): string | undefined {
	const normalized = path.replace(/[\\/]+$/u, '');
	const separator = Math.max(normalized.lastIndexOf('/'), normalized.lastIndexOf('\\'));
	if (separator < 0) return undefined;
	if (separator === 0) return normalized[0];
	if (separator === 2 && /^[A-Za-z]:[\\/]/u.test(normalized)) return normalized.slice(0, 3);
	return normalized.slice(0, separator);
}

function resourceExtension(filename: string): string {
	const dot = filename.lastIndexOf('.');
	return dot > 0 ? filename.slice(dot) : '';
}

function resourceLanguageId(input: NonNullable<IEditorPart['activeInput']>, resolver: TextResourceLanguageResolver | undefined): string | undefined {
	if (input.languageId) return input.languageId;
	return isTextResourceLanguageInput(input, resolver) ? resolveTextResourceLanguageId(input, resolver) : undefined;
}

function bufferContextKeyChanges<T>(contextKeyService: IContextKeyService, callback: () => T): T {
	let result!: T;
	contextKeyService.bufferChangeEvents(() => {
		result = callback();
	});
	return result;
}
