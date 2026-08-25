import "./media/editorpart.css";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import type { URI } from "../../../../base/common/uri.js";
import { Emitter, type Event } from '../../../../base/common/event.js';
import { Dimension, type IDimension } from "../../../../base/browser/geometry.js";
import { observeElementSize } from "../../../../base/browser/observer.js";
import { SplitView, type ISplitViewView } from "../../../../base/browser/ui/splitview/splitview.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import type { IKeybindingService } from "../../../../platform/keybinding/common/keybinding.js";
import type { IKeybindingsResourceService } from "../../../../platform/keybinding/common/keybindingsResource.js";
import type { IKeyboardLayoutService } from "../../../../platform/keyboardLayout/common/keyboardLayout.js";
import type { IContextKeyService } from "../../../../platform/contextkey/common/contextkey.js";
import { type ITextFileService } from "../../../services/textfile/common/textFileService.js";
import type { IFileService } from "../../../../platform/files/common/files.js";
import { type ITextMateService } from "../../../services/textMate/common/textMateService.js";
import { type ILanguageFeaturesService } from "../../../services/language/common/languageFeaturesService.js";
import type { IDiffApi } from "../../../../platform/diff/common/diffApi.js";
import type { IInstantiationService } from "../../../../platform/instantiation/common/instantiation.js";
import type { ISyntaxApi } from "../../../../platform/syntax/common/syntaxApi.js";
import type { IDocumentCollaborationApi } from "../../../../platform/collaboration/common/documentCollaborationApi.js";
import type { IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import { WorkbenchPart } from "../../part.js";
import { EditorGroup, type EditorGroupOptions, type IEditorGroup } from "./editorGroup.js";
import { EditorTabDragAndDropController, type EditorTabDropEvent } from "./editorTabDragAndDrop.js";
import type { EditorInput, EditorOpenOptions, EditorOpenTarget } from "../../../services/editor/common/editorService.js";
import type { TextResourceLanguageResolver } from "../../../../platform/language/common/textResourceLanguage.js";
import type { IWorkingCopyService } from "../../../services/workingCopy/common/workingCopyService.js";
import type { IEditorPane } from "./editorPane.js";
import { EditorPaneRegistry, EditorPanes } from "./editorRegistry.js";
import type { IBulkEditService } from "../../../contrib/bulkEdit/common/bulkEdit.js";
import type { ILanguageDiagnosticsService } from "../../../../editor/common/services/languageDiagnosticsService.js";
import type { EditorLineGutterDecoration } from "../../../../editor/browser/viewparts/margin/lineGutterDecoration.js";
import type { OwnedDecorationSource } from "../../../../editor/browser/viewparts/decorations/decorationPresentation.js";
import type { TextModel } from "../../../../editor/common/model/textModel.js";
import type { EditorWelcomeOptions, IEditorWelcomeProject } from "../../../contrib/files/browser/editorWelcome.js";
import { EditorInputSerializers, type EditorInputSerializerRegistry, isSerializedEditorInput } from "../../../services/editor/common/editorInputSerializer.js";
import type { ApplyEditorWorkingSetOptions, EditorWorkingSet, EditorWorkingSetTarget } from "../../../services/editor/common/editorWorkingSet.js";
import { ModalEditorPart } from "./modalEditorPart.js";

export { EditorOpenSupersededError } from "./editorGroup.js";

/** Editor-region operations available to Workbench contributions. */
export interface IEditorPart {
	readonly onDidChangeEditors: Event<void>;
	readonly domNode: HTMLElement;
	readonly groups: readonly IEditorGroup[];
	readonly activeGroup: IEditorGroup;
	readonly activeInput: EditorInput | undefined;
	readonly activePane: IEditorPane | undefined;
	readonly isModalEditorVisible: boolean;

	openEditor(input: EditorInput, options?: EditorOpenOptions, target?: EditorOpenTarget): Promise<IEditorPane>;
	activateEditor(input: EditorInput): IEditorPane;
	closeEditor(input: EditorInput): void;
	setWelcomeRecentProjects(projects: readonly IEditorWelcomeProject[]): void;
	setWelcomeVisible(visible: boolean): void;
	saveActiveEditor(): Promise<void>;
	setContent(content: Element): void;
	splitActiveGroupHorizontal(): Promise<void>;
	saveWorkingSet(id: string): EditorWorkingSet;
	applyWorkingSet(workingSet: EditorWorkingSetTarget, options?: ApplyEditorWorkingSetOptions): Promise<void>;
	layout(dimension: IDimension): void;
	focus(): void;
}

export const IEditorPart =
	createServiceIdentifier<IEditorPart>("editorPart");

/** Named collaborators used to construct the editor region. */
export interface IEditorPartOptions {
	readonly configurationService?: IConfigurationService;
	readonly contextKeyService?: IContextKeyService;
	readonly keybindingService?: IKeybindingService;
	readonly keybindingsResourceService?: IKeybindingsResourceService;
	readonly keyboardLayoutService?: IKeyboardLayoutService;
	readonly fileService?: IFileService;
	readonly textFileService?: ITextFileService;
	readonly textMateService?: ITextMateService;
	readonly languageFeaturesService?: ILanguageFeaturesService;
	readonly languageResolver?: TextResourceLanguageResolver;
	readonly diffApi?: IDiffApi;
	readonly instantiationService?: IInstantiationService;
	readonly syntaxApi?: ISyntaxApi;
	readonly languageDiagnosticsService?: ILanguageDiagnosticsService;
	readonly documentCollaborationApi?: IDocumentCollaborationApi;
	readonly serverEvents?: IServerEventApi;
	readonly workingCopyService?: IWorkingCopyService;
	readonly bulkEditService?: IBulkEditService;
	readonly createLineGutterDecorations?: (resource: URI) => readonly EditorLineGutterDecoration[];
	readonly createDecorationSources?: (resource: URI, model: TextModel) => readonly OwnedDecorationSource[];
	readonly registry?: EditorPaneRegistry;
	readonly titleActions?: {
		readonly menuService: IMenuService;
		readonly contextMenuProvider: IContextMenuProvider;
	};
	readonly welcome?: EditorWelcomeOptions;
	readonly welcomeVisible?: boolean;
	readonly saveAsResource?: (defaultName: string) => Promise<URI | undefined>;
	readonly inputSerializers?: EditorInputSerializerRegistry;
}

/** Owns EditorGroup layout and delegates editor behavior to the active group. */
export class EditorPart extends WorkbenchPart implements IEditorPart {
	private readonly editorsChangeEmitter = this.own(new Emitter<void>());
	private readonly splitView: SplitView;
	private readonly modalEditor: ModalEditorPart;
	private readonly groupOptions: Omit<EditorGroupOptions, "onDidActivate" | "onDidChangeEditors" | "dragAndDrop">;
	private readonly _groups: EditorGroupHost[] = [];
	private _activeGroup: EditorGroup;
	private readonly tabDragAndDrop: EditorTabDragAndDropController;
	private welcomeRecentProjects: readonly IEditorWelcomeProject[];
	private dimension = Dimension.Zero;
	private readonly saveAsResource: ((defaultName: string) => Promise<URI | undefined>) | undefined;
	private readonly inputSerializers: EditorInputSerializerRegistry;
	readonly onDidChangeEditors = this.editorsChangeEmitter.event;

	override get minimumWidth(): number { return 120; }
	override get minimumHeight(): number { return 119; }

	constructor(
		container: HTMLElement,
		options: IEditorPartOptions = {},
	) {
		super(container, "editor");
		const ownerDocument = container.ownerDocument;
		this.titleDomNode.remove();
		this.domNode.setAttribute("aria-label", "Editor");
		this.groupOptions = {
			registry: options.registry ?? EditorPanes,
			configurationService: options.configurationService,
			contextKeyService: options.contextKeyService,
			keybindingService: options.keybindingService,
			keybindingsResourceService: options.keybindingsResourceService,
			keyboardLayoutService: options.keyboardLayoutService,
			fileService: options.fileService,
			textFileService: options.textFileService,
			textMateService: options.textMateService,
			languageFeaturesService: options.languageFeaturesService,
			languageResolver: options.languageResolver,
			diffApi: options.diffApi,
			instantiationService: options.instantiationService,
			syntaxApi: options.syntaxApi,
			languageDiagnosticsService: options.languageDiagnosticsService,
			documentCollaborationApi: options.documentCollaborationApi,
			serverEvents: options.serverEvents,
			workingCopyService: options.workingCopyService,
			onOpenLocation: location => this.openEditor({ resource: location.resource }, { selection: location.selectionRange ?? location.range }).then(() => undefined),
			onApplyWorkspaceEdit: options.bulkEditService ? edit => options.bulkEditService!.apply(edit).then(() => undefined) : undefined,
			createLineGutterDecorations: options.createLineGutterDecorations,
			createDecorationSources: options.createDecorationSources,
			titleActions: options.titleActions,
			welcome: options.welcome,
			welcomeVisible: options.welcomeVisible,
			...(options.saveAsResource ? {
				onSave: (group: IEditorGroup, input: EditorInput, pane: IEditorPane) => this.saveEditor(group, input, pane),
			} : {}),
		};
		this.welcomeRecentProjects = options.welcome?.recentProjects ?? [];
		this.saveAsResource = options.saveAsResource;
		this.inputSerializers = options.inputSerializers ?? EditorInputSerializers;
		this.tabDragAndDrop = new EditorTabDragAndDropController((event) => {
			this.dropEditor(event);
		});
		this.splitView = this.own(new SplitView(
			this.contentDomNode,
			"horizontal",
		));
		const initial = this.createGroup();
		this._groups.push(initial);
		this._activeGroup = initial.group;
		this.splitView.addView(initial.view);
		this.modalEditor = this.own(new ModalEditorPart({
			container,
			registry: this.groupOptions.registry,
			paneCreationOptions: {
				configurationService: options.configurationService,
				contextKeyService: options.contextKeyService,
				keybindingService: options.keybindingService,
				keybindingsResourceService: options.keybindingsResourceService,
				keyboardLayoutService: options.keyboardLayoutService,
				fileService: options.fileService,
				textFileService: options.textFileService,
				textMateService: options.textMateService,
				languageFeaturesService: options.languageFeaturesService,
				diffApi: options.diffApi,
				instantiationService: options.instantiationService,
				syntaxApi: options.syntaxApi,
				languageDiagnosticsService: options.languageDiagnosticsService,
				documentCollaborationApi: options.documentCollaborationApi,
				serverEvents: options.serverEvents,
				workingCopyService: options.workingCopyService,
				onOpenLocation: location => this.openEditor({ resource: location.resource }, { selection: location.selectionRange ?? location.range }).then(() => undefined),
				onApplyWorkspaceEdit: options.bulkEditService ? edit => options.bulkEditService!.apply(edit).then(() => undefined) : undefined,
				createLineGutterDecorations: options.createLineGutterDecorations,
				createDecorationSources: options.createDecorationSources,
				...(options.titleActions ? {
					actionServices: {
						menuService: options.titleActions.menuService,
						contextMenuProvider: options.titleActions.contextMenuProvider,
						contextKeyService: options.contextKeyService,
					},
				} : {}),
			},
		}));
		this.own(this.modalEditor.onDidRequestClose(input => this.closeEditor(input)));
		this.notifyEditorsChanged();
		this.own(observeElementSize(this.contentDomNode, size => this.layout(size)));
	}

	get groups(): readonly IEditorGroup[] {
		return this._groups.map(({ group }) => group);
	}

	get activeGroup(): IEditorGroup {
		return this._activeGroup;
	}

	get activeInput(): EditorInput | undefined {
		if (this.modalEditor.isVisible) return this.modalEditor.activeInput;
		return this._activeGroup.activeInput;
	}

	get activePane(): IEditorPane | undefined {
		if (this.modalEditor.isVisible) return this.modalEditor.activePane;
		return this._activeGroup.activePane;
	}

	get isModalEditorVisible(): boolean {
		return this.modalEditor.isVisible;
	}

	async openEditor(input: EditorInput, options: EditorOpenOptions = {}, target: EditorOpenTarget = "activeGroup"): Promise<IEditorPane> {
		if (target === "modalGroup") {
			const pane = await this.modalEditor.openEditor(input, options);
			this.notifyEditorsChanged();
			return pane;
		}
		this.closeActiveModalEditor();
		if (target === "activeGroup") return this._activeGroup.openEditor(input, options);
		const source = this._activeGroup;
		const { host, created } = this.resolveSideGroup(source);
		try {
			const pane = await host.group.openEditor(input, options);
			if (!options.preserveFocus) {
				this._activeGroup = host.group;
				this.notifyEditorsChanged();
			}
			return pane;
		} catch (error) {
			if (created) this.removeGroup(host);
			this._activeGroup = source;
			this.notifyEditorsChanged();
			throw error;
		}
	}

	activateEditor(input: EditorInput): IEditorPane {
		if (this.modalEditor.activeInput?.resource.toString() === input.resource.toString()) {
			this.modalEditor.focus();
			return this.modalEditor.activePane!;
		}
		return this._activeGroup.activateEditor(input);
	}

	closeEditor(input: EditorInput): void {
		if (this.modalEditor.closeEditor(input)) {
			this.notifyEditorsChanged();
			return;
		}
		this._activeGroup.closeEditor(input);
	}

	setWelcomeRecentProjects(projects: readonly IEditorWelcomeProject[]): void {
		this.welcomeRecentProjects = projects;
		for (const { group } of this._groups) group.setWelcomeRecentProjects(projects);
	}

	setWelcomeVisible(visible: boolean): void {
		for (const { group } of this._groups) group.setWelcomeVisible(visible);
	}

	async saveActiveEditor(): Promise<void> {
		await this.activePane?.save?.();
	}

	setContent(content: Element): void {
		this.closeActiveModalEditor();
		this._activeGroup.setContent(content);
	}

	async splitActiveGroupHorizontal(): Promise<void> {
		const source = this._activeGroup;
		const created = this.insertGroupAfter(source);
		this._activeGroup = created.group;
		this.notifyEditorsChanged();
		try {
			if (source.activeInput) {
				await created.group.openEditor(source.activeInput);
			}
			created.group.focus();
		} catch (error) {
			this.removeGroup(created);
			this._activeGroup = source;
			this.notifyEditorsChanged();
			throw error;
		}
	}

	override layout(dimension: IDimension): void {
		this.dimension = new Dimension(dimension.width, dimension.height);
		this.splitView.layout(
			this.dimension.width,
			this.dimension.height,
		);
	}

	focus(): void {
		if (this.modalEditor.isVisible) {
			this.modalEditor.focus();
			return;
		}
		this._activeGroup.focus();
	}

	private closeActiveModalEditor(): void {
		const input = this.modalEditor.activeInput;
		if (!input || !this.modalEditor.closeEditor(input)) return;
		this.notifyEditorsChanged();
	}

	private async saveEditor(group: IEditorGroup, input: EditorInput, pane: IEditorPane): Promise<boolean> {
		if (input.resource.scheme !== "untitled") throw new Error("Save As is only available for untitled editors");
		if (!this.saveAsResource) throw new Error("Editor Save As is unavailable in this host");
		if (!pane.saveAs) throw new Error("The active editor cannot save this document");
		const target = await this.saveAsResource(editorInputLabel(input));
		if (!target) return false;
		await pane.saveAs(target);
		await group.replaceEditor(input, {
			resource: target,
			label: editorInputLabel({ resource: target }),
		});
		return true;
	}

	private createGroup(): EditorGroupHost {
		let group: EditorGroup;
		group = this.own(new EditorGroup(this.contentDomNode, {
			...this.groupOptions,
			...(this.groupOptions.welcome ? {
				welcome: { ...this.groupOptions.welcome, recentProjects: this.welcomeRecentProjects },
			} : {}),
			onDidActivate: () => {
				this._activeGroup = group;
				this.notifyEditorsChanged();
			},
			onDidChangeEditors: () => {
				this.notifyEditorsChanged();
			},
			dragAndDrop: {
				start: (source, input) => this.tabDragAndDrop.start(source, input),
				isDragging: () => this.tabDragAndDrop.isDragging(),
				drop: (target, targetInput, position) => this.tabDragAndDrop.drop(target, targetInput, position),
				end: () => this.tabDragAndDrop.end(),
			},
		}));
		return {
			group,
			view: new EditorGroupSplitView(group),
		};
	}

	private resolveSideGroup(source: EditorGroup): { readonly host: EditorGroupHost; readonly created: boolean } {
		const sourceIndex = this.groupIndex(source);
		const existing = this._groups[sourceIndex + 1];
		if (existing) return { host: existing, created: false };
		return { host: this.insertGroupAfter(source), created: true };
	}

	private insertGroupAfter(source: EditorGroup): EditorGroupHost {
		const sourceIndex = this.groupIndex(source);
		const created = this.createGroup();
		const targetIndex = sourceIndex + 1;
		this._groups.splice(targetIndex, 0, created);
		this.splitView.addView(created.view, { type: "split", index: sourceIndex }, targetIndex);
		this.splitView.distributeViewSizes();
		return created;
	}

	private removeGroup(host: EditorGroupHost): void {
		const index = this._groups.indexOf(host);
		if (index < 0) return;
		this.splitView.removeView(index);
		this._groups.splice(index, 1);
		host.group.dispose();
	}

	private groupIndex(group: EditorGroup): number {
		const index = this._groups.findIndex((host) => host.group === group);
		if (index < 0) throw new Error("EditorGroup is not owned by EditorPart");
		return index;
	}

	private dropEditor(event: EditorTabDropEvent): void {
		const targetIndex = event.target.getEditorInsertionIndex(event.targetInput, event.position);
		if (event.source === event.target) {
			event.target.moveEditor(event.input, targetIndex);
			this._activeGroup = event.target;
			this.notifyEditorsChanged();
			event.target.focus();
			return;
		}
		void event.source.moveEditorTo(event.input, event.target, targetIndex)
			.then(() => {
				this._activeGroup = event.target;
				this.notifyEditorsChanged();
				event.target.focus();
			})
			.catch((error) => {
				console.error("Failed to move Editor tab", error);
			});
	}

	saveWorkingSet(id: string): EditorWorkingSet {
		if (!id.trim()) throw new TypeError("Editor working set requires a non-empty ID");
		const totalSize = this._groups.reduce((sum, _host, index) => sum + this.splitView.getViewSize(index), 0);
		return Object.freeze({
			id,
			activeGroupIndex: this._groups.findIndex(({ group }) => group === this._activeGroup),
			groups: Object.freeze(this._groups.map(({ group }, index) => Object.freeze({
				editors: Object.freeze(group.inputs.map(input => Object.freeze({
					input: this.inputSerializers.serialize(input),
					preview: group.isPreview(input),
				}))),
				activeEditorIndex: group.activeInput ? group.inputs.indexOf(group.activeInput) : -1,
				size: totalSize > 0 ? this.splitView.getViewSize(index) / totalSize : 1 / this._groups.length,
			}))),
		});
	}

	async applyWorkingSet(workingSet: EditorWorkingSetTarget, options: ApplyEditorWorkingSetOptions = {}): Promise<void> {
		const target = workingSet === "empty" ? emptyWorkingSet() : validateWorkingSet(workingSet);
		const groups = target.groups.map(group => ({
			...group,
			inputs: group.editors.map(editor => this.inputSerializers.deserialize(editor.input)),
		}));
		const hadEditorFocus = this.domNode.contains(this.domNode.ownerDocument.activeElement);
		for (const host of [...this._groups]) {
			for (const input of [...host.group.inputs]) host.group.closeEditor(input);
		}
		while (this._groups.length > 1) this.removeGroup(this._groups[this._groups.length - 1]!);
		while (this._groups.length < groups.length) this.insertGroupAfter(this._groups[this._groups.length - 1]!.group);
		for (let groupIndex = 0; groupIndex < groups.length; groupIndex += 1) {
			const state = groups[groupIndex]!;
			const group = this._groups[groupIndex]!.group;
			for (let inputIndex = 0; inputIndex < state.inputs.length; inputIndex += 1) {
				await group.openEditor(state.inputs[inputIndex]!, {
					index: inputIndex,
					pinned: !state.editors[inputIndex]!.preview,
					preserveFocus: true,
				});
			}
			const activeInput = state.inputs[state.activeEditorIndex];
			if (activeInput) group.activateEditor(activeInput);
		}
		const activeGroup = this._groups[target.activeGroupIndex] ?? this._groups[0]!;
		this._activeGroup = activeGroup.group;
		this.notifyEditorsChanged();
		const availableSize = Math.max(0, this.dimension.width - Math.max(0, groups.length - 1));
		for (let index = 0; index < groups.length; index += 1) {
			this.splitView.resizeView(index, availableSize * groups[index]!.size);
		}
		if (!options.preserveFocus && hadEditorFocus) this._activeGroup.focus();
	}

	private notifyEditorsChanged(): void {
		this.editorsChangeEmitter.fire();
	}
}

function emptyWorkingSet(): EditorWorkingSet {
	return Object.freeze({
		id: "empty",
		activeGroupIndex: 0,
		groups: Object.freeze([Object.freeze({ editors: Object.freeze([]), activeEditorIndex: -1, size: 1 })]),
	});
}

function validateWorkingSet(value: EditorWorkingSet): EditorWorkingSet {
	if (!value || typeof value !== "object" || typeof value.id !== "string" || !value.id.trim()) {
		throw new TypeError("Invalid editor working set");
	}
	if (!Array.isArray(value.groups) || value.groups.length === 0 || !Number.isInteger(value.activeGroupIndex) || value.activeGroupIndex < 0 || value.activeGroupIndex >= value.groups.length) {
		throw new TypeError("Invalid editor working set groups");
	}
	let sizeTotal = 0;
	for (const group of value.groups) {
		if (!group || typeof group !== "object" || !Array.isArray(group.editors) || !Number.isInteger(group.activeEditorIndex) || group.activeEditorIndex < -1 || group.activeEditorIndex >= group.editors.length || !Number.isFinite(group.size) || group.size < 0) {
			throw new TypeError("Invalid editor group working set");
		}
		for (const editor of group.editors) {
			if (!editor || typeof editor !== "object" || typeof editor.preview !== "boolean" || !isSerializedEditorInput(editor.input)) {
				throw new TypeError("Invalid editor working set entry");
			}
		}
		sizeTotal += group.size;
	}
	if (sizeTotal <= 0) throw new TypeError("Invalid editor working set layout");
	return value;
}

function editorInputLabel(input: Pick<EditorInput, "resource" | "label">): string {
	if (input.label?.trim()) return input.label;
	const path = decodeURIComponent(input.resource.path).replace(/\/+$/, "");
	const separator = path.lastIndexOf("/");
	return path.slice(separator + 1) || input.resource.toString();
}

interface EditorGroupHost {
	readonly group: EditorGroup;
	readonly view: EditorGroupSplitView;
}

class EditorGroupSplitView implements ISplitViewView {
	readonly minimumSize = 120;
	readonly maximumSize = Infinity;

	constructor(readonly group: EditorGroup) {}

	get element(): HTMLElement {
		return this.group.domNode;
	}

	layout(size: number, _offset: number, orthogonalSize: number): void {
		this.group.layout({
			width: size,
			height: orthogonalSize,
		});
	}
}
