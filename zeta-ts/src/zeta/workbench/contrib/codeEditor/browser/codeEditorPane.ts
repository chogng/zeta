import "./media/editorPane.css";
import { addDisposableListener, stopEvent, h } from "../../../../base/browser/dom.js";
import { type IDimension } from "../../../../base/browser/geometry.js";
import { throwIfCancelled } from "../../../../base/common/cancellation.js";
import { Disposable, DisposableStore, MutableDisposable, type IDisposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { assertDefined } from "../../../../base/common/types.js";
import type { URI } from "../../../../base/common/uri.js";
import { type ITextMateService } from "../../../services/textMate/common/textMateService.js";
import type { ILanguageFeaturesService } from '../../../../editor/common/services/languageFeatures.js';
import type { ILanguageConfigurationService } from '../../../../editor/common/services/languageConfigurationService.js';
import type { TextResourceLanguageResolver } from '../../../../platform/language/common/textResourceLanguage.js';
import { type EditorInput } from "../../../browser/parts/editor/editorInput.js";
import { type IEditorPane } from "../../../browser/parts/editor/editorPane.js";
import { EditorPaneVisibility } from "../../../browser/parts/editor/editorPane.js";
import { CODE_EDITOR_ID, languageForEditorInput } from "./codeEditorInput.js";
import { type ITextResourceStore } from "../../../../editor/common/services/textResourceStore.js";
import { EditorBrowser, isEditorTextViewState, type EditorBrowserOptions, type EditorTextViewState } from "../../../../editor/browser/editorBrowser.js";
import { type ITextModelService, type TextModelReference } from "../../../../editor/common/services/textModelService.js";
import { type EditorTextDirection } from "../../../../editor/browser/view.js";
import { type EditorLineWrapping, type WrappingIndent } from "../../../../editor/common/config/editorOptions.js";
import { type IWorkingCopy, type IWorkingCopyService } from "../../../services/workingCopy/common/workingCopyService.js";
import { type TextRange } from "../../../../editor/common/core/text.js";
import { type LanguageLocation } from "../../../../editor/contrib/gotoSymbol/common/languageNavigation.js";
import { type LanguageWorkspaceEdit } from "../../../../editor/common/languages/languageWorkspaceEdit.js";
import { type ILanguageDiagnosticsService } from "../../../../editor/common/services/languageDiagnosticsService.js";
import { type OwnedDecorationSource } from "../../../../editor/browser/viewparts/decorations/decorationPresentation.js";
import { type TextModel } from "../../../../editor/common/model/textModel.js";
import type { EditorSelectionController } from "../../../../editor/common/cursor/editorSelectionController.js";
import type { EditorPaneStatus } from "../../../browser/parts/editor/editorPane.js";
import type { IAccessibilityService } from "../../../../platform/accessibility/common/accessibility.js";

export interface EditorPanePart extends IDisposable {
	readonly onDidChange?: Event<void>;
	readonly selections?: EditorSelectionController;
	layout(dimension: IDimension): void;
	focus(): void;
	getValue(): string;
	revealRange?(range: TextRange): void;
	getViewState?(): EditorTextViewState;
	restoreViewState?(state: EditorTextViewState): void;
	announceAccessibilityStatus?(message: string): void;
	prepareSave?(): Promise<void>;
}

export interface EditorPanePartOptions extends EditorBrowserOptions {
	readonly textMateService?: ITextMateService;
	readonly languageFeaturesService?: ILanguageFeaturesService;
	readonly languageConfigurationService?: ILanguageConfigurationService;
	readonly languageResolver?: TextResourceLanguageResolver;
	readonly languageDiagnosticsService?: ILanguageDiagnosticsService;
	readonly instantiationService?: EditorBrowserOptions["instantiationService"];
	readonly accessibilityService?: IAccessibilityService;
}

export interface EditorPaneOptions {
	readonly modelService: ITextModelService;
	readonly workingCopyService?: IWorkingCopyService;
	readonly createPart?: (options: EditorPanePartOptions) => EditorPanePart;
	readonly textMateService?: ITextMateService;
	readonly languageFeaturesService?: ILanguageFeaturesService;
	readonly languageConfigurationService?: ILanguageConfigurationService;
	readonly languageResolver?: TextResourceLanguageResolver;
	readonly languageDiagnosticsService?: ILanguageDiagnosticsService;
	readonly instantiationService?: EditorBrowserOptions["instantiationService"];
	readonly accessibilityService?: IAccessibilityService;
	readonly lineWrapping?: EditorLineWrapping;
	readonly wrappingIndent?: WrappingIndent;
	readonly fontFamily?: string;
	readonly fontSize?: number;
	readonly lineHeight?: number;
	readonly fontLigatures?: boolean;
	readonly experimentalGpuAcceleration?: EditorBrowserOptions["experimentalGpuAcceleration"];
	readonly minimap?: EditorBrowserOptions["minimap"];
	readonly activeLineHighlight?: EditorBrowserOptions["activeLineHighlight"];
	readonly showLineNumbers?: boolean;
	readonly showIndentationGuides?: boolean;
	readonly bracketPairColorization?: boolean;
	readonly stickyScroll?: boolean;
	readonly suggestions?: boolean;
	readonly inlineCompletions?: boolean;
	readonly parameterHints?: boolean;
	readonly inlayHints?: boolean;
	readonly codeLens?: boolean;
	readonly formatOnSave?: boolean;
	readonly find?: EditorBrowserOptions["find"];
	readonly indentation?: EditorBrowserOptions["indentation"];
	/** Browser paragraph direction forwarded to every created editor part. */
	readonly textDirection?: EditorTextDirection;
	readonly onOpenLink?: (target: string) => void | Promise<void>;
	readonly onShowContextMenu?: EditorBrowserOptions["onShowContextMenu"];
	readonly onExecuteEditorCommand?: EditorBrowserOptions["onExecuteEditorCommand"];
	readonly onOpenLocation?: (location: LanguageLocation) => void | Promise<void>;
	readonly onApplyWorkspaceEdit?: (edit: LanguageWorkspaceEdit) => void | Promise<void>;
	readonly createDecorationSources?: (resource: URI, model: TextModel) => readonly OwnedDecorationSource[];
	readonly placeholder?: string;
	readonly showUnicodeHighlights?: boolean;
	readonly insertFinalNewLine?: boolean;
	readonly fontZoom?: EditorBrowserOptions["fontZoom"];
	readonly onSave?: () => Promise<void | boolean>;
	readonly onSaveError?: (error: unknown) => void;
}

/** Workbench pane that composes the text model, input, view, and language services. */
export class CodeEditorPane extends Disposable implements IEditorPane {
	readonly id = CODE_EDITOR_ID;
	readonly viewStateTypeId = "stanza.code.textView";
	private readonly workingCopySlot = this._register(new MutableDisposable<IWorkingCopy>());
	private readonly part = this._register(new MutableDisposable<EditorPanePart>());
	private readonly statusListener = this._register(new MutableDisposable<IDisposable>());
	private readonly statusChangeEmitter = this._register(new Emitter<void>());
	private readonly modelService: ITextModelService;
	private readonly createPart: (options: EditorPanePartOptions) => EditorPanePart;
	private container: HTMLDivElement | undefined;
	private dimension: IDimension = { width: 0, height: 0 };
	private saving = false;
	private languageId: string | undefined;
	readonly onDidChangeStatus = this.statusChangeEmitter.event;

	get workingCopy(): IWorkingCopy | undefined {
		return this.workingCopySlot.value;
	}

	constructor(private readonly resourceStore: ITextResourceStore, private readonly options: EditorPaneOptions) {
		super();
		if (!resourceStore || typeof resourceStore.resolve !== "function" || typeof resourceStore.save !== "function" || typeof resourceStore.onDidChange !== "function") {
			this.dispose();
			throw new TypeError("Code editor pane requires a text resource store");
		}
		if (!options || !options.modelService || typeof options.modelService.acquire !== "function") {
			this.dispose();
			throw new TypeError("Code editor pane requires a text model service");
		}
		this.modelService = options.modelService;
		this.createPart = options.createPart ?? (partOptions => new EditorBrowser(partOptions));
	}

	create(parent: HTMLElement): void {
		if (this.container) throw new ReferenceError("EditorPane has already been created");
		const container = h(parent.ownerDocument, "div");
		container.className = "stanza-editor-pane";
		parent.append(container);
		this.container = container;
		this._register(addDisposableListener<KeyboardEvent>(container, "keydown", event => this.handleSaveKeydown(event)));
		this._register(toDisposable(() => {
			container.remove();
			this.container = undefined;
		}));
	}

	async setInput(input: EditorInput, signal: AbortSignal): Promise<void> {
		const container = this.requireContainer();
		throwIfCancelled(signal, "Code editor input loading was cancelled");
		const modelReference = await this.modelService.acquire(input, signal);
		let part: EditorPanePart | undefined;
		let workingCopy: EditorWorkingCopy | undefined;
		try {
			throwIfCancelled(signal, "Code editor input loading was cancelled");
			const languageId = languageForEditorInput({ ...input, firstLine: modelReference.model.getLineContent(0) }, this.options.languageResolver);
			part = this.createPart({
				container,
				input,
				languageId,
				model: modelReference.model,
				textMateService: this.options.textMateService,
				languageFeaturesService: this.options.languageFeaturesService,
				languageConfigurationService: this.options.languageConfigurationService,
				languageDiagnosticsService: this.options.languageDiagnosticsService,
				instantiationService: this.options.instantiationService,
				accessibilityService: this.options.accessibilityService,
				lineWrapping: this.options.lineWrapping,
				wrappingIndent: this.options.wrappingIndent,
				fontFamily: this.options.fontFamily,
				fontSize: this.options.fontSize,
				lineHeight: this.options.lineHeight,
				fontLigatures: this.options.fontLigatures,
				experimentalGpuAcceleration: this.options.experimentalGpuAcceleration,
				minimap: this.options.minimap,
				activeLineHighlight: this.options.activeLineHighlight,
				showLineNumbers: this.options.showLineNumbers,
				showIndentationGuides: this.options.showIndentationGuides,
				bracketPairColorization: this.options.bracketPairColorization,
				stickyScroll: this.options.stickyScroll,
				suggestions: this.options.suggestions,
				inlineCompletions: this.options.inlineCompletions,
				parameterHints: this.options.parameterHints,
				inlayHints: this.options.inlayHints,
				codeLens: this.options.codeLens,
				formatOnSave: this.options.formatOnSave,
				find: this.options.find,
				indentation: this.options.indentation,
				textDirection: this.options.textDirection,
				onOpenLink: this.options.onOpenLink,
				onShowContextMenu: this.options.onShowContextMenu,
				onExecuteEditorCommand: this.options.onExecuteEditorCommand,
				onOpenLocation: this.options.onOpenLocation,
				onApplyWorkspaceEdit: this.options.onApplyWorkspaceEdit,
				decorationSources: this.options.createDecorationSources?.(input.resource, modelReference.model),
				placeholder: this.options.placeholder,
				showUnicodeHighlights: this.options.showUnicodeHighlights,
				insertFinalNewLine: this.options.insertFinalNewLine,
				fontZoom: this.options.fontZoom,
			});
			workingCopy = new EditorWorkingCopy(
				modelReference,
				this.resourceStore,
				input,
				this.options.workingCopyService,
				input.resource.scheme === "untitled" ? this.options.onSave : undefined,
			);
			throwIfCancelled(signal, "Code editor input loading was cancelled");
		} catch (error) {
			part?.dispose();
			workingCopy?.dispose();
			if (!workingCopy) modelReference.dispose();
			throw error;
		}
		this.statusListener.clear();
		this.part.value = part;
		this.workingCopySlot.value = workingCopy;
		this.languageId = languageForEditorInput({ ...input, firstLine: modelReference.model.getLineContent(0) }, this.options.languageResolver);
		const statusListeners = new DisposableStore();
		if (part.onDidChange) statusListeners.add(part.onDidChange(() => this.statusChangeEmitter.fire()));
		statusListeners.add(modelReference.onDidChangeExternalChange(() => {
			if (modelReference.hasExternalChange) part.announceAccessibilityStatus?.("File changed on disk. Local edits are preserved.");
			this.statusChangeEmitter.fire();
		}));
		this.statusListener.value = statusListeners;
		part.layout(this.dimension);
		this.statusChangeEmitter.fire();
	}

	clearInput(): void {
		this.statusListener.clear();
		this.part.clear();
		this.workingCopySlot.clear();
		this.languageId = undefined;
		this.statusChangeEmitter.fire();
	}

	layout(dimension: IDimension): void {
		this.dimension = {
			width: Math.max(0, dimension.width),
			height: Math.max(0, dimension.height),
		};
		this.part.value?.layout(this.dimension);
	}

	setVisible(visibility: EditorPaneVisibility): void {
		if (!this.container) return;
		this.container.hidden = visibility === EditorPaneVisibility.Hidden;
		if (visibility === EditorPaneVisibility.Visible) this.part.value?.layout(this.dimension);
	}

	focus(): void {
		this.part.value?.focus();
	}

	getValue(): string {
		return this.part.value?.getValue() ?? "";
	}

	async saveAs(resource: URI): Promise<void> {
		const workingCopy = this.workingCopy;
		if (workingCopy) {
			await workingCopy.saveAs(resource, new AbortController().signal);
			return;
		}
		await this.resourceStore.save({ resource, text: this.getValue() }, new AbortController().signal);
	}

	get isDirty(): boolean {
		return this.workingCopy?.isDirty ?? false;
	}

	get hasExternalChange(): boolean {
		return this.workingCopy?.hasExternalChange ?? false;
	}

	async save(): Promise<void> {
		await this.part.value?.prepareSave?.();
		await this.workingCopy?.save(new AbortController().signal);
	}

	async revert(): Promise<void> {
		await this.workingCopy?.revert(new AbortController().signal);
	}

	revealRange(range: TextRange): void {
		this.part.value?.revealRange?.(range);
	}

	saveViewState(): unknown {
		return this.part.value?.getViewState?.();
	}

	restoreViewState(state: unknown): void {
		if (!isEditorTextViewState(state)) throw new TypeError("Invalid Stanza code editor view state");
		const part = this.part.value;
		if (!part?.restoreViewState) throw new Error("Stanza code editor view-state restoration is unavailable");
		part.restoreViewState(state);
	}

	getStatus(): EditorPaneStatus {
		const selections = this.part.value?.selections?.selections;
		const active = selections?.primary.active;
		return Object.freeze({
			...(active ? { lineNumber: active.lineIndex + 1, columnNumber: active.columnIndex + 1 } : {}),
			...(selections && selections.selections.length > 1 ? { selectionCount: selections.selections.length } : {}),
			...(this.languageId ? { languageId: this.languageId } : {}),
			encoding: "UTF-8",
			endOfLine: "LF",
		});
	}

	private handleSaveKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
		if ((!event.ctrlKey && !event.metaKey) || event.shiftKey || event.altKey || event.key.toLowerCase() !== "s") return;
		stopEvent(event);
		if (this.saving) return;
		this.saving = true;
		void this.save().then(() => {
			this.part.value?.announceAccessibilityStatus?.("Saved");
		}).catch(error => {
			const message = error instanceof Error && error.message.trim().length > 0 ? error.message.trim() : "unknown error";
			this.part.value?.announceAccessibilityStatus?.(`Save failed: ${message}`);
			(this.options.onSaveError ?? reportSaveError)(error);
		}).finally(() => {
			this.saving = false;
		});
	}

	private requireContainer(): HTMLDivElement {
		assertDefined(this.container, new ReferenceError("EditorPane has not been created"));
		return this.container;
	}
}

function reportSaveError(error: unknown): void {
	console.error("Code editor save failed", error);
}

class EditorWorkingCopy extends Disposable implements IWorkingCopy {
	readonly resource: URI;
	readonly backupKind = "text" as const;
	readonly backupLanguageId: string | undefined;
	readonly backupContentType: string | undefined;
	readonly backupLabel: string | undefined;
	readonly onDidChangeDirty: IWorkingCopy["onDidChangeDirty"];
	readonly onDidChangeExternalChange: IWorkingCopy["onDidChangeExternalChange"];
	readonly onDidChangeContent: IWorkingCopy["onDidChangeContent"];

	constructor(
		private readonly reference: TextModelReference,
		private readonly resourceStore: ITextResourceStore,
		input: EditorInput,
		workingCopyService: IWorkingCopyService | undefined,
		private readonly saveUntitled: (() => Promise<void | boolean>) | undefined,
	) {
		super();
		this._register(reference);
		this.resource = input.resource;
		this.backupLanguageId = input.languageId;
		this.backupContentType = input.contentType;
		this.backupLabel = input.label;
		this.onDidChangeDirty = reference.onDidChangeDirty;
		this.onDidChangeExternalChange = reference.onDidChangeExternalChange;
		this.onDidChangeContent = listener => reference.model.onDidChange(() => listener());
		if (workingCopyService) this._register(workingCopyService.register(this));
	}

	get isDirty(): boolean {
		return this.reference.isDirty;
	}

	get hasExternalChange(): boolean {
		return this.reference.hasExternalChange;
	}

	backup(): string {
		return this.reference.model.getText();
	}

	restoreBackup(content: string): void {
		this.reference.model.reset(content);
	}

	save(signal: AbortSignal): Promise<void> {
		throwIfCancelled(signal, "Code editor working-copy save was cancelled");
		if (this.resource.scheme === "untitled") return this.saveUntitledDocument();
		return this.reference.save(signal);
	}

	async saveAs(resource: URI, signal: AbortSignal): Promise<void> {
		await this.resourceStore.save({ resource, text: this.reference.model.getText() }, signal);
	}

	revert(signal: AbortSignal): Promise<void> {
		return this.reference.revert(signal);
	}

	private async saveUntitledDocument(): Promise<void> {
		const result = await this.saveUntitled?.();
		if (result === false) return;
		if (!this.saveUntitled) throw new Error("Untitled code editor has no save handler");
	}
}
