import { raceCancellation, throwIfCancelled } from "../../../../base/common/cancellation.js";
import { type Event } from "../../../../base/common/event.js";
import { type URI } from "../../../../base/common/uri.js";
import { type IFileChangeEvent, type IFileService } from "../../../../platform/files/common/files.js";
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
}

export interface TextFileSaveRequest {
  readonly resource: URI;
  readonly text: string;
}

/** Resource-content boundary used by text editors independently of their model implementation. */
export interface ITextFileService {
  readonly onDidChangeFiles: Event<IFileChangeEvent>;
  resolve(request: TextFileResolveRequest, signal: AbortSignal): Promise<ResolvedTextFileContent>;
  save(request: TextFileSaveRequest, signal: AbortSignal): Promise<void>;
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
      });
    }
    const text = await raceCancellation(this.files.readFile(request.resource), signal, "Text file resolution was cancelled");
    if (typeof text !== "string") throw new TypeError("File service returned non-text content");
    return Object.freeze({
      resource: request.resource,
      text,
      source: TextFileContentSource.FileSystem,
    });
  }

  async save(request: TextFileSaveRequest, signal: AbortSignal): Promise<void> {
    validateSaveRequest(request);
    throwIfCancelled(signal, "Text file save was cancelled");
    await raceCancellation(this.files.writeFile(request.resource, request.text), signal, "Text file save was cancelled");
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
}
