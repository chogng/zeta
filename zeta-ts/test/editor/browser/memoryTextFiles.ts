import { Emitter } from "../../../src/zeta/base/common/event.js";
import type { URI } from "../../../src/zeta/base/common/uri.js";
import type { IFileChangeEvent } from "../../../src/zeta/platform/files/common/files.js";
import { TextFileContentSource } from "../../../src/zeta/workbench/services/textfile/common/textFileService.js";
import { TextFileSaveConflictError } from "../../../src/zeta/workbench/services/textfile/common/textFileService.js";
import type { ITextFileService } from "../../../src/zeta/workbench/services/textfile/common/textFileService.js";
import type { ResolvedTextFileContent } from "../../../src/zeta/workbench/services/textfile/common/textFileService.js";
import type { TextFileResolveRequest } from "../../../src/zeta/workbench/services/textfile/common/textFileService.js";
import type { TextFileSaveRequest } from "../../../src/zeta/workbench/services/textfile/common/textFileService.js";

/** Browser-only in-memory text-file service used by editor integration pages. */
export class MemoryTextFiles implements ITextFileService {
  private readonly changes = new Emitter<IFileChangeEvent>();
  private readonly contents = new Map<string, string>();
  private readonly revisions = new Map<string, number>();

  readonly onDidChangeFiles = this.changes.event;

  constructor(resource: URI, text: string) {
    this.contents.set(resource.toString(), text);
    this.revisions.set(resource.toString(), 1);
  }

  async resolve(request: TextFileResolveRequest, _signal: AbortSignal): Promise<ResolvedTextFileContent> {
    return Object.freeze({
      resource: request.resource,
      text: request.bootstrapText ?? this.contents.get(request.resource.toString()) ?? "",
      source: request.bootstrapText === undefined ? TextFileContentSource.FileSystem : TextFileContentSource.Bootstrap,
      revision: request.bootstrapText === undefined ? this.revisionFor(request.resource) : undefined,
    });
  }

  async save(request: TextFileSaveRequest, _signal: AbortSignal): Promise<{ readonly revision: string | undefined }> {
    if (request.expectedRevision !== undefined && request.expectedRevision !== this.revisionFor(request.resource)) {
      throw new TextFileSaveConflictError(request.resource);
    }
    this.contents.set(request.resource.toString(), request.text);
    this.revisions.set(request.resource.toString(), (this.revisions.get(request.resource.toString()) ?? 0) + 1);
    this.changes.fire({ resources: [request.resource] });
    return { revision: this.revisionFor(request.resource) };
  }

  read(resource: URI): string {
    return this.contents.get(resource.toString()) ?? "";
  }

  dispose(): void {
    this.changes.dispose();
    this.contents.clear();
    this.revisions.clear();
  }

  private revisionFor(resource: URI): string {
    return `revision-${this.revisions.get(resource.toString()) ?? 0}`;
  }
}
