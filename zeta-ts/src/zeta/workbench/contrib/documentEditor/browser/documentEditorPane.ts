import { assertDefined } from "../../../../base/common/types.js";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import type { IDimension } from "../../../../base/browser/geometry.js";
import type { URI } from "../../../../base/common/uri.js";
import { RichTextEditorWidget, type RichTextEditorOptions } from "../../../../editor/browser/widget/richTextEditor/richTextEditorWidget.js";
import type { DocumentSelection } from "../../../../editor/common/core/documentSelection.js";
import type { DocumentNode } from "../../../../editor/common/model/document.js";
import type { DocumentOutline } from "../../../../editor/common/model/documentOutline.js";
import { EditorPaneVisibility, type IEditorPane } from "../../../browser/parts/editor/editorPane.js";
import type { EditorInput } from "../../../browser/parts/editor/editorInput.js";
import { DocumentEditorTextModelService } from "../../../services/documentEditor/browser/documentEditorTextModelService.js";
import type { ITextFileService } from "../../../services/textfile/common/textFileService.js";
import type { IWorkingCopy } from "../../../services/workingCopy/common/workingCopyService.js";
import type { IWorkingCopyService } from "../../../services/workingCopy/common/workingCopyService.js";
import { DOCUMENT_EDITOR_ID } from "./documentEditorInput.js";
import { h } from "../../../../base/browser/dom.js";

/** Workbench-only services that complement one document editor. */
export interface EditorPaneOptions extends RichTextEditorOptions {
	readonly workingCopyService?: IWorkingCopyService;
}

/** Workbench pane that hosts one structured document editor. */
export class DocumentEditorPane extends Disposable implements IEditorPane {
	readonly id = DOCUMENT_EDITOR_ID;

	private readonly editor: RichTextEditorWidget;
	private container: HTMLDivElement | undefined;
	private dimension: IDimension = { width: 0, height: 0 };

	get workingCopy(): IWorkingCopy | undefined {
		return this.editor.modelReference;
	}

	constructor(textFiles: ITextFileService, options: EditorPaneOptions = {}) {
		super();
		const modelService = this._register(new DocumentEditorTextModelService(textFiles, options.workingCopyService));
		const collaborationService = options.documentCollaborationService ? this._register(options.documentCollaborationService) : undefined;
		this.editor = this._register(new RichTextEditorWidget(modelService, { ...options, ...(collaborationService ? { documentCollaborationService: collaborationService } : {}) }));
		this._register(toDisposable(() => {
			this.container?.remove();
			this.container = undefined;
		}));
	}

	create(parent: HTMLElement): void {
		if (this.container) throw new ReferenceError("Document editor pane has already been created");
		const container = h(parent.ownerDocument, "div");
		container.className = "stanza-structured-editor-pane";
		parent.append(container);
		this.container = container;
		this.editor.create(container);
	}

	async setInput(input: EditorInput, signal: AbortSignal): Promise<void> {
		this.requireContainer();
		await this.editor.setInput(input, signal);
		this.editor.layout(this.dimension);
	}

	clearInput(): void {
		this.editor.clearInput();
	}

	layout(dimension: IDimension): void {
		this.dimension = { width: Math.max(0, dimension.width), height: Math.max(0, dimension.height) };
		this.editor.layout(this.dimension);
	}

	setVisible(visibility: EditorPaneVisibility): void {
		if (this.container) this.container.hidden = visibility === EditorPaneVisibility.Hidden;
	}

	focus(): void {
		this.editor.focus();
	}

	async save(): Promise<void> {
		await this.editor.save();
	}

	async saveAs(resource: URI): Promise<void> {
		await this.editor.saveAs(resource);
	}

	async revert(): Promise<void> {
		await this.editor.revert();
	}

	get isDirty(): boolean {
		return this.editor.isDirty;
	}

	get hasExternalChange(): boolean {
		return this.editor.hasExternalChange;
	}

	getDocument(): DocumentNode {
		return this.editor.getDocument();
	}

	/** Returns the current structured-document selection of the hosted editor. */
	getDocumentSelection(): DocumentSelection | undefined {
		return this.editor.getDocumentSelection();
	}

	getOutline(): DocumentOutline {
		return this.editor.getOutline();
	}

	private requireContainer(): HTMLDivElement {
		assertDefined(this.container, new ReferenceError("Document editor pane has not been created"));
		return this.container;
	}
}
