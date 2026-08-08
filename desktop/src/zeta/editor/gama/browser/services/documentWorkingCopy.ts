import { throwIfCancelled } from "../../../../base/common/cancellation.js";
import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { TextFileSaveConflictError } from "../../../../workbench/services/textfile/common/textFileService.js";
import type { ITextFileService } from "../../../../workbench/services/textfile/common/textFileService.js";
import type { IWorkingCopy } from "../../../../workbench/services/workingCopy/common/workingCopyService.js";
import type { IWorkingCopyService } from "../../../../workbench/services/workingCopy/common/workingCopyService.js";
import { documentFromPlainText } from "../../common/model/documentText.js";
import { type DocumentNode } from "../../common/model/document.js";
import { DocumentModel } from "../../common/model/documentModel.js";
import { createDefaultDocumentSchema, type DocumentSchema } from "../../common/model/documentSchema.js";
import { DocumentSerializationError, deserializeDocument, serializeDocument } from "../../common/model/documentSerialization.js";

export interface DocumentWorkingCopyOptions {
  readonly resource: URI;
  readonly model: DocumentModel;
  readonly initialDocument: DocumentNode;
  readonly initialRevision: string | undefined;
  readonly textFiles: ITextFileService;
  readonly workingCopyService?: IWorkingCopyService;
  readonly onSave?: () => Promise<void | boolean>;
  /** Creates the canonical document when the persisted resource is empty. */
  readonly createEmptyDocument?: () => DocumentNode;
}

/** Persistence adapter for Gama's immutable document model. */
export class DocumentWorkingCopy extends DisposableOwner implements IWorkingCopy {
  private readonly dirtyEmitter = this.own(new Emitter<void>());
  private readonly externalChangeEmitter = this.own(new Emitter<void>());
  private readonly schema: DocumentSchema;
  private readonly initialDocument: DocumentNode;
  private readonly initialContent: string;
  private savedContent: string;
  private revision: string | undefined;
  private dirty = false;
  private externalChange = false;

  readonly resource: URI;
  readonly onDidChangeDirty = this.dirtyEmitter.event;
  readonly onDidChangeExternalChange = this.externalChangeEmitter.event;

  constructor(private readonly options: DocumentWorkingCopyOptions) {
    super();
    this.resource = options.resource;
    this.schema = options.model.schema;
    this.initialDocument = options.initialDocument;
    this.initialContent = serializeDocument(options.initialDocument, this.schema);
    this.savedContent = this.initialContent;
    this.revision = options.initialRevision;
    this.own(options.model.onDidChange(() => this.refreshDirty()));
    this.own(options.textFiles.onDidChangeFiles(event => {
      if (this.resource.scheme === "untitled" || (event.resources && !event.resources.some(resource => resource.toString() === this.resource.toString()))) return;
      if (this.isDirty) {
        this.setExternalChange(true);
        return;
      }
      void this.reloadCleanDocument();
    }));
    if (options.workingCopyService) this.own(options.workingCopyService.register(this));
  }

  get isDirty(): boolean {
    return this.dirty;
  }

  get hasExternalChange(): boolean {
    return this.externalChange;
  }

  async save(signal: AbortSignal): Promise<void> {
    throwIfCancelled(signal, "Document save was cancelled");
    if (this.resource.scheme === "untitled") {
      if (!this.options.onSave) throw new Error("Untitled document has no save handler");
      const result = await this.options.onSave();
      if (result === false) return;
      this.savedContent = serializeDocument(this.options.model.document, this.schema);
      this.refreshDirty();
      this.setExternalChange(false);
      return;
    }
    const serialized = serializeDocument(this.options.model.document, this.schema);
    let saved;
    try {
      saved = await this.options.textFiles.save({ resource: this.resource, text: serialized, ...(this.revision === undefined ? {} : { expectedRevision: this.revision }) }, signal);
    } catch (error) {
      if (error instanceof TextFileSaveConflictError) this.setExternalChange(true);
      throw error;
    }
    this.savedContent = serialized;
    this.revision = saved.revision;
    this.refreshDirty();
    this.setExternalChange(false);
  }

  async saveAs(resource: URI, signal: AbortSignal): Promise<void> {
    throwIfCancelled(signal, "Document Save As was cancelled");
    const serialized = serializeDocument(this.options.model.document, this.schema);
    await this.options.textFiles.save({ resource, text: serialized }, signal);
    this.savedContent = serialized;
    this.refreshDirty();
    this.setExternalChange(false);
  }

  async revert(signal: AbortSignal): Promise<void> {
    throwIfCancelled(signal, "Document revert was cancelled");
    if (this.resource.scheme === "untitled") {
      this.savedContent = this.initialContent;
      this.options.model.reset(this.initialDocument);
      this.setExternalChange(false);
      return;
    }
    const content = await this.options.textFiles.resolve({ resource: this.resource }, signal);
    throwIfCancelled(signal, "Document revert was cancelled");
    const document = parseDocument(content.text, this.schema, this.options.createEmptyDocument);
    this.savedContent = serializeDocument(document, this.schema);
    this.revision = content.revision;
    this.options.model.reset(document);
    this.setExternalChange(false);
  }

  private async reloadCleanDocument(): Promise<void> {
    try {
      const content = await this.options.textFiles.resolve({ resource: this.resource }, new AbortController().signal);
      if (this.isDirty) {
        this.setExternalChange(true);
        return;
      }
      const document = parseDocument(content.text, this.schema, this.options.createEmptyDocument);
      this.savedContent = serializeDocument(document, this.schema);
      this.revision = content.revision;
      this.options.model.reset(document);
      this.setExternalChange(false);
    } catch {
      this.setExternalChange(true);
    }
  }

  private refreshDirty(): void {
    const dirty = serializeDocument(this.options.model.document, this.schema) !== this.savedContent;
    if (this.dirty === dirty) return;
    this.dirty = dirty;
    this.dirtyEmitter.fire();
  }

  private setExternalChange(value: boolean): void {
    if (this.externalChange === value) return;
    this.externalChange = value;
    this.externalChangeEmitter.fire();
  }
}

/** Parses Gama's versioned format and migrates plain text into paragraphs. */
export function parseDocument(text: string, schema: DocumentSchema = createDefaultDocumentSchema(), createEmptyDocument?: () => DocumentNode): DocumentNode {
  if (text.trim().length === 0) {
    const document = createEmptyDocument?.() ?? schema.createDocument([]);
    schema.validate(document);
    return document;
  }
  if (text.trimStart().startsWith("{")) return deserializeDocument(text, schema);
  try {
    return deserializeDocument(text, schema);
  } catch (error) {
    if (!(error instanceof DocumentSerializationError)) throw error;
    return documentFromPlainText(schema, text);
  }
}
