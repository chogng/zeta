import { throwIfCancelled } from "../../../../base/common/cancellation.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { TextModel } from "../../../../editor/common/model/textModel.js";
import type { ITextModelService, TextModelStructureInput, TextModelWorkingCopyReference } from "../../../../editor/common/services/textModelService.js";
import type { ITextFileService } from "../../textfile/common/textFileService.js";
import type { IWorkingCopyService } from "../../workingCopy/common/workingCopyService.js";
import { DocumentWorkingCopy } from "./documentWorkingCopy.js";
import { parseDocument } from "./documentWorkingCopy.js";

/** Workbench persistence adapter for a TextModel opened by the document editor. */
export class DocumentEditorTextModelService extends DisposableOwner implements ITextModelService<TextModelStructureInput, TextModelWorkingCopyReference> {
	constructor(private readonly textFiles: ITextFileService, private readonly workingCopyService?: IWorkingCopyService) {
		super();
		if (!textFiles || typeof textFiles.resolve !== "function" || typeof textFiles.save !== "function") {
			this.dispose();
			throw new TypeError("Stanza document editor TextModel service requires a Workbench text file service");
		}
	}

	async acquire(input: TextModelStructureInput, signal: AbortSignal): Promise<TextModelWorkingCopyReference> {
		throwIfCancelled(signal, "Stanza document editor TextModel acquisition was cancelled");
		const content = await this.textFiles.resolve({
			resource: input.resource,
			...(input.initialText === undefined ? {} : { bootstrapText: input.initialText }),
		}, signal);
		throwIfCancelled(signal, "Stanza document editor TextModel acquisition was cancelled");
		const document = parseDocument(content.text, input.schema, input.createEmptyDocument);
		const model = TextModel.createWithStructure(input.schema, document, { plugins: input.plugins });
		const workingCopy = new DocumentWorkingCopy({
			resource: input.resource,
			model,
			initialDocument: document,
			initialRevision: content.revision,
			textFiles: this.textFiles,
			workingCopyService: this.workingCopyService,
			onSave: input.onSave,
			createEmptyDocument: input.createEmptyDocument,
		});
		return new TextModelWorkingCopyReferenceImpl(model, workingCopy);
	}
}

class TextModelWorkingCopyReferenceImpl extends DisposableOwner implements TextModelWorkingCopyReference {
	readonly resource;
	readonly model;
	readonly onDidChangeDirty;
	readonly onDidChangeExternalChange;
	readonly onDidChangeContent;
	readonly backupKind;
	readonly backupContentType;

	constructor(model: TextModel, private readonly workingCopy: DocumentWorkingCopy) {
		super();
		this.model = this.own(model);
		this.workingCopy = this.own(workingCopy);
		this.resource = workingCopy.resource;
		this.onDidChangeDirty = workingCopy.onDidChangeDirty;
		this.onDidChangeExternalChange = workingCopy.onDidChangeExternalChange;
		this.onDidChangeContent = workingCopy.onDidChangeContent;
		this.backupKind = workingCopy.backupKind;
		this.backupContentType = workingCopy.backupContentType;
	}

	get isDirty(): boolean {
		return this.workingCopy.isDirty;
	}

	get hasExternalChange(): boolean {
		return this.workingCopy.hasExternalChange;
	}

	backup(): string {
		return this.workingCopy.backup();
	}

	restoreBackup(content: string): void {
		this.workingCopy.restoreBackup(content);
	}

	save(signal: AbortSignal): Promise<void> {
		return this.workingCopy.save(signal);
	}

	saveAs(resource: TextModelWorkingCopyReference["resource"], signal: AbortSignal): Promise<void> {
		return this.workingCopy.saveAs(resource, signal);
	}

	revert(signal: AbortSignal): Promise<void> {
		return this.workingCopy.revert(signal);
	}
}
