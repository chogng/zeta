import { throwIfCancelled } from "../../../../base/common/cancellation.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { DocumentModel } from "../../../../editor/common/model/documentModel.js";
import type { DocumentModelInput } from "../../../../editor/common/services/documentModelService.js";
import type { DocumentModelReference } from "../../../../editor/common/services/documentModelService.js";
import type { IDocumentModelService } from "../../../../editor/common/services/documentModelService.js";
import type { ITextFileService } from "../../textfile/common/textFileService.js";
import type { IWorkingCopyService } from "../../workingCopy/common/workingCopyService.js";
import { DocumentWorkingCopy } from "./documentWorkingCopy.js";
import { parseDocument } from "./documentWorkingCopy.js";

/** Browser implementation of Aster's structured document-model service. */
export class BrowserDocumentModelService extends DisposableOwner implements IDocumentModelService {
  constructor(private readonly textFiles: ITextFileService, private readonly workingCopyService?: IWorkingCopyService) {
    super();
    if (!textFiles || typeof textFiles.resolve !== "function" || typeof textFiles.save !== "function") {
      this.dispose();
      throw new TypeError("Aster document model service requires a Workbench text file service");
    }
  }

  async acquire(input: DocumentModelInput, signal: AbortSignal): Promise<DocumentModelReference> {
    throwIfCancelled(signal, "Aster document model acquisition was cancelled");
    const content = await this.textFiles.resolve({
      resource: input.resource,
      ...(input.initialText === undefined ? {} : { bootstrapText: input.initialText }),
    }, signal);
    throwIfCancelled(signal, "Aster document model acquisition was cancelled");
    const document = parseDocument(content.text, input.schema, input.createEmptyDocument);
    const model = new DocumentModel(input.schema, document, { plugins: input.plugins });
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
    return new BrowserDocumentModelReference(model, workingCopy);
  }
}

class BrowserDocumentModelReference extends DisposableOwner implements DocumentModelReference {
  readonly resource;
  readonly model;
  readonly onDidChangeDirty;
  readonly onDidChangeExternalChange;

  constructor(model: DocumentModel, private readonly workingCopy: DocumentWorkingCopy) {
    super();
    this.model = this.own(model);
    this.workingCopy = this.own(workingCopy);
    this.resource = workingCopy.resource;
    this.onDidChangeDirty = workingCopy.onDidChangeDirty;
    this.onDidChangeExternalChange = workingCopy.onDidChangeExternalChange;
  }

  get isDirty(): boolean {
    return this.workingCopy.isDirty;
  }

  get hasExternalChange(): boolean {
    return this.workingCopy.hasExternalChange;
  }

  save(signal: AbortSignal): Promise<void> {
    return this.workingCopy.save(signal);
  }

  saveAs(resource: DocumentModelReference["resource"], signal: AbortSignal): Promise<void> {
    return this.workingCopy.saveAs(resource, signal);
  }

  revert(signal: AbortSignal): Promise<void> {
    return this.workingCopy.revert(signal);
  }
}
