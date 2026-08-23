import { raceCancellation, throwIfCancelled } from "../../../../base/common/cancellation.js";
import { type Event } from "../../../../base/common/event.js";
import { type URI } from "../../../../base/common/uri.js";
import { FileRevisionConflictError, type IFileChangeEvent, type IFileService } from "../../../../platform/files/common/files.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

export enum TextFileContentSource {
  Bootstrap = "bootstrap",
  FileSystem = "fileSystem",
}

export interface TextFileResolveRequest {
  readonly resource: URI;
  readonly bootstrapText?: string;
}

export interface ResolvedTextFileContent {
  readonly resource: URI;
  readonly text: string;
  readonly source: TextFileContentSource;
  /** Opaque file revision when the content came from the workspace file service. */
  readonly revision: string | undefined;
}

export interface TextFileSaveRequest {
  readonly resource: URI;
  readonly text: string;
  readonly expectedRevision?: string;
}

/** Result of one successful text-file save. */
export interface TextFileSaveResult {
  readonly revision: string | undefined;
}

/** A conditional file save was rejected because the resource changed after it was resolved. */
export class TextFileSaveConflictError extends Error {
  constructor(readonly resource: URI) {
    super(`Text file changed since it was resolved: ${resource.toString()}`);
    this.name = "TextFileSaveConflictError";
  }
}

/** Resource-content boundary used by text editors independently of their model implementation. */
export interface ITextFileService {
  readonly onDidChangeFiles: Event<IFileChangeEvent>;
  resolve(request: TextFileResolveRequest, signal: AbortSignal): Promise<ResolvedTextFileContent>;
  save(request: TextFileSaveRequest, signal: AbortSignal): Promise<TextFileSaveResult>;
}

export const ITextFileService = createServiceIdentifier<ITextFileService>("textFileService");

/** Resolves bootstrap snapshots first and otherwise delegates workspace reads to the file service. */
export class TextFileService implements ITextFileService {
  readonly onDidChangeFiles: Event<IFileChangeEvent>;

  constructor(private readonly files: IFileService) {
    if (!files || typeof files.readFile !== "function" || typeof files.writeFile !== "function") {
      throw new TypeError("Text file service requires a file service");
    }
    this.onDidChangeFiles = files.onDidChangeFiles;
  }

  async resolve(request: TextFileResolveRequest, signal: AbortSignal): Promise<ResolvedTextFileContent> {
    validateRequest(request);
    throwIfCancelled(signal, "Text file resolution was cancelled");
    if (request.bootstrapText !== undefined) {
      return Object.freeze({
        resource: request.resource,
        text: request.bootstrapText,
        source: TextFileContentSource.Bootstrap,
        revision: undefined,
      });
    }
    const content = await raceCancellation(this.files.readFile(request.resource), signal, "Text file resolution was cancelled");
    if (typeof content.content !== "string" || typeof content.revision !== "string") throw new TypeError("File service returned invalid text content");
    return Object.freeze({
      resource: request.resource,
      text: content.content,
      source: TextFileContentSource.FileSystem,
      revision: content.revision,
    });
  }

  async save(request: TextFileSaveRequest, signal: AbortSignal): Promise<TextFileSaveResult> {
    validateSaveRequest(request);
    throwIfCancelled(signal, "Text file save was cancelled");
    try {
      const saved = await raceCancellation(this.files.writeFile({
        resource: request.resource,
        content: request.text,
        ...(request.expectedRevision === undefined ? {} : { expectedRevision: request.expectedRevision }),
      }), signal, "Text file save was cancelled");
      return Object.freeze({ revision: saved.revision });
    } catch (error) {
      if (error instanceof FileRevisionConflictError) throw new TextFileSaveConflictError(request.resource);
      throw error;
    }
  }
}

function validateRequest(request: TextFileResolveRequest): void {
  if (!request || typeof request !== "object" || !request.resource || typeof request.resource.toString !== "function") {
    throw new TypeError("Text file resolution requires a resource");
  }
  if (request.bootstrapText !== undefined && typeof request.bootstrapText !== "string") {
    throw new TypeError("Text file bootstrap content must be text");
  }
}

function validateSaveRequest(request: TextFileSaveRequest): void {
  if (!request || typeof request !== "object" || !request.resource || typeof request.resource.toString !== "function") {
    throw new TypeError("Text file save requires a resource");
  }
  if (typeof request.text !== "string") {
    throw new TypeError("Text file save content must be text");
  }
  if (request.expectedRevision !== undefined && typeof request.expectedRevision !== "string") {
    throw new TypeError("Text file expected revision must be text");
  }
}
