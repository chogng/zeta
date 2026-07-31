import { raceCancellation, throwIfCancelled } from "../../../../base/common/cancellation.js";
import { type URI } from "../../../../base/common/uri.js";
import { type IFileService } from "../../../../platform/files/common/files.js";
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

/** Resource-content boundary used by text editors independently of their model implementation. */
export interface ITextFileService {
  resolve(request: TextFileResolveRequest, signal: AbortSignal): Promise<ResolvedTextFileContent>;
}

export const ITextFileService = createServiceIdentifier<ITextFileService>("textFileService");

/** Resolves bootstrap snapshots first and otherwise delegates workspace reads to the file service. */
export class TextFileService implements ITextFileService {
  constructor(private readonly files: IFileService) {
    if (!files || typeof files.readFile !== "function") {
      throw new TypeError("Text file service requires a file service");
    }
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
}

function validateRequest(request: TextFileResolveRequest): void {
  if (!request || typeof request !== "object" || !request.resource || typeof request.resource.toString !== "function") {
    throw new TypeError("Text file resolution requires a resource");
  }
  if (request.bootstrapText !== undefined && typeof request.bootstrapText !== "string") {
    throw new TypeError("Text file bootstrap content must be text");
  }
}
