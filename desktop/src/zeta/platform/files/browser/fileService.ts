import type { FsFileType, FsGetMetadataParams, FsGetMetadataResult, FsReadBinaryFileParams, FsReadBinaryFileResult, FsReadDirectoryParams, FsReadDirectoryResult, FsReadFileParams, FsReadFileResult, FsWriteFileParams, FsWriteFileResult, ResourceMetadataResult, ResourceReadResult } from "../../../../../generated/app-server/types.js";
import type { FsChanged } from "../../../../../generated/app-server/types.js";
import type { IResourceApi } from "../../app-server/common/appServerApi.js";
import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { URI } from "../../../base/common/uri.js";
import { FileKind, FileNotFoundError, FileRevisionConflictError, type FileDeleteMode, type FileExistingTargetBehavior, type FileMissingTargetBehavior, type IFileBytes, type IFileChangeEvent, type IFileContent, type IFileEntry, type IFileService, type IFileStat, type IFileWriteRequest, type IFileWriteResult } from "../common/files.js";
import type { IWorkspaceContextService } from "../../workspace/common/workspace.js";

/** Narrow App Server surface consumed by the browser file-service adapter. */
export interface IFileSystemApi {
  getMetadata(params: FsGetMetadataParams): Promise<FsGetMetadataResult>;
  readDirectory(params: FsReadDirectoryParams): Promise<FsReadDirectoryResult>;
  readFile(params: FsReadFileParams): Promise<FsReadFileResult>;
  readBinaryFile(params: FsReadBinaryFileParams): Promise<FsReadBinaryFileResult>;
  writeFile(params: FsWriteFileParams): Promise<FsWriteFileResult>;
  createFile(params: import("../../../../../generated/app-server/types.js").FsCreateFileParams): Promise<FsGetMetadataResult>;
  rename(params: import("../../../../../generated/app-server/types.js").FsRenameParams): Promise<void>;
  delete(params: import("../../../../../generated/app-server/types.js").FsDeleteParams): Promise<void>;
}

export interface BrowserFileServiceOptions {
  readonly api: IFileSystemApi;
  readonly resourceApi: IResourceApi;
  readonly workspaceContextService: IWorkspaceContextService;
  readonly onDidChange?: Event<FsChanged>;
}

/**
 * Maps workspace resource URIs to the App Server's root-relative filesystem protocol.
 */
export class BrowserFileService extends DisposableOwner implements IFileService {
  private readonly api: IFileSystemApi;
  private readonly resourceApi: IResourceApi;
  private readonly workspaceContextService: IWorkspaceContextService;
  private readonly fileChanges = this.own(new Emitter<IFileChangeEvent>());

  readonly onDidChangeFiles = this.fileChanges.event;

  constructor(options: BrowserFileServiceOptions) {
    super();
    this.api = options.api;
    this.resourceApi = options.resourceApi;
    this.workspaceContextService = options.workspaceContextService;
    if (options.onDidChange) this.own(options.onDidChange(change => this.acceptFileChange(change)));
  }

  async stat(resource: URI): Promise<IFileStat> {
    let result;
    try { result = await this.api.getMetadata({ path: this.relativePath(resource) }); }
    catch (error) { if (isFileNotFound(error)) throw new FileNotFoundError(resource); throw error; }
    return {
      resource,
      kind: fileKind(result.fileType),
      sizeBytes: result.sizeBytes,
      readonly: result.readonly,
      modifiedAtMillis: result.modifiedAtMillis ?? undefined,
    };
  }

  async readDirectory(resource: URI): Promise<readonly IFileEntry[]> {
    const result = await this.api.readDirectory({
      path: this.relativePath(resource),
    });
    return result.entries.map((entry) => ({
      resource: childResource(resource, entry.name),
      name: entry.name,
      kind: fileKind(entry.fileType),
    }));
  }

  async readFile(resource: URI): Promise<IFileContent> {
    const result = await this.api.readFile({
      path: this.relativePath(resource),
    });
    return Object.freeze({ resource, content: result.content, revision: result.revision });
  }

  async readFileBytes(resource: URI): Promise<IFileBytes> {
    const result = await this.api.readBinaryFile({
      path: this.relativePath(resource),
    });
    try {
      const bytes = await this.readResourceBytes(result.resource);
      return Object.freeze({ resource, bytes, revision: result.revision });
    } finally {
      await this.resourceApi.release({ resourceId: result.resource.resourceId });
    }
  }

  async writeFile(request: IFileWriteRequest): Promise<IFileWriteResult> {
    try {
      const result = await this.api.writeFile({
        path: this.relativePath(request.resource),
        content: request.content,
        ...(request.expectedRevision === undefined ? {} : { expectedRevision: request.expectedRevision }),
      });
      return Object.freeze({
        stat: {
          resource: request.resource,
          kind: fileKind(result.metadata.fileType),
          sizeBytes: result.metadata.sizeBytes,
          readonly: result.metadata.readonly,
          modifiedAtMillis: result.metadata.modifiedAtMillis ?? undefined,
        },
        revision: result.revision,
      });
    } catch (error) {
      if (isRevisionConflict(error)) throw new FileRevisionConflictError(request.resource);
      throw error;
    }
  }

  async createFile(resource: URI, existing: FileExistingTargetBehavior): Promise<IFileStat> {
    const result = await this.api.createFile({ path: this.relativePath(resource), existing });
    return { resource, kind: fileKind(result.fileType), sizeBytes: result.sizeBytes, readonly: result.readonly, modifiedAtMillis: result.modifiedAtMillis ?? undefined };
  }

  rename(source: URI, target: URI, existing: FileExistingTargetBehavior): Promise<void> {
    return this.api.rename({ source: this.relativePath(source), target: this.relativePath(target), existing });
  }

  delete(resource: URI, missing: FileMissingTargetBehavior, mode: FileDeleteMode): Promise<void> {
    return this.api.delete({ path: this.relativePath(resource), missing, mode });
  }

  private relativePath(resource: URI): string {
    const folders = this.workspaceContextService.getWorkspace().folders;
    if (folders.length !== 1) {
      throw new Error(
        "The current filesystem protocol requires one workspace folder",
      );
    }
    return workspaceRelativePath(folders[0].uri, resource);
  }

  private async readResourceBytes(resource: ResourceMetadataResult): Promise<Uint8Array> {
    if (!Number.isSafeInteger(resource.size) || resource.size < 0 || resource.size > MAX_BINARY_FILE_BYTES) {
      throw new Error("Workspace binary resource size is invalid");
    }
    const bytes = new Uint8Array(resource.size);
    let offset = 0;
    while (offset < bytes.length) {
      const chunk = await this.resourceApi.read({
        resourceId: resource.resourceId,
        offset,
        maxBytes: Math.min(MAX_RESOURCE_READ_BYTES, bytes.length - offset),
      });
      const chunkBytes = decodeResourceChunk(chunk, resource.resourceId, offset, bytes.length);
      bytes.set(chunkBytes, offset);
      offset += chunkBytes.length;
    }
    return bytes;
  }

  private acceptFileChange(change: FsChanged): void {
    if (change.type === "rescanRequired") {
      this.fileChanges.fire(Object.freeze({ resources: undefined }));
      return;
    }
    const folders = this.workspaceContextService.getWorkspace().folders;
    if (folders.length !== 1) {
      this.fileChanges.fire(Object.freeze({ resources: undefined }));
      return;
    }
    const resources = change.paths.map(path => workspaceResourceFromPath(folders[0].uri, path));
    if (resources.some(resource => resource === undefined)) {
      this.fileChanges.fire(Object.freeze({ resources: undefined }));
      return;
    }
    const unique = new Map<string, URI>();
    for (const resource of resources) unique.set(resource!.toString(), resource!);
    this.fileChanges.fire(Object.freeze({ resources: Object.freeze([...unique.values()]) }));
  }
}

function isRevisionConflict(error: unknown): boolean {
  return error instanceof Error && error.message === "FileSystemRevisionConflict";
}

function isFileNotFound(error: unknown): boolean {
  return error instanceof Error && error.message === "FileSystemNotFound";
}

const MAX_RESOURCE_READ_BYTES = 262_144;
const MAX_BINARY_FILE_BYTES = 16 * 1024 * 1024;

function decodeResourceChunk(chunk: ResourceReadResult, resourceId: string, expectedOffset: number, totalSize: number): Uint8Array {
  if (chunk.resourceId !== resourceId || chunk.offset !== expectedOffset) {
    throw new Error("Workspace binary resource response is inconsistent");
  }
  const binary = atob(chunk.dataBase64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  if (chunk.decodedLength !== bytes.length || bytes.length === 0 || bytes.length > totalSize - expectedOffset || chunk.eof !== (expectedOffset + bytes.length === totalSize)) {
    throw new Error("Workspace binary resource response is inconsistent");
  }
  return bytes;
}

/** Resolves a resource to a slash-separated path beneath one workspace root. */
export function workspaceRelativePath(root: URI, resource: URI): string {
  if (
    root.scheme !== "file" ||
    resource.scheme !== "file" ||
    root.authority.toLowerCase() !== resource.authority.toLowerCase()
  ) {
    throw new Error("Resource must belong to the local workspace filesystem");
  }
  const rootPath = decodedPath(root).replace(/\/+$/, "");
  const resourcePath = decodedPath(resource).replace(/\/+$/, "");
  const ignoreCase = isCaseInsensitiveFileSystemPath(rootPath);
  const comparedRoot = ignoreCase ? rootPath.toLowerCase() : rootPath;
  const comparedResource = ignoreCase
    ? resourcePath.toLowerCase()
    : resourcePath;
  if (comparedResource === comparedRoot) return ".";
  const prefix = `${comparedRoot}/`;
  if (!comparedResource.startsWith(prefix)) {
    throw new Error("Resource is outside the current workspace folder");
  }
  return resourcePath.slice(rootPath.length + 1);
}

function decodedPath(resource: URI): string {
  return decodeURIComponent(resource.path).replaceAll("\\", "/");
}

function isCaseInsensitiveFileSystemPath(path: string): boolean {
  return /^\/[A-Za-z]:\//.test(`${path}/`) ||
    globalThis.navigator?.platform?.startsWith("Mac") === true;
}

function childResource(parent: URI, name: string): URI {
  const base = parent.path.endsWith("/")
    ? parent.path.slice(0, -1)
    : parent.path;
  return parent.withPath(`${base}/${encodeURIComponent(name)}`);
}

/** Resolves one slash-separated protocol path beneath a workspace root. */
export function workspaceResourceFromPath(root: URI, path: string): URI | undefined {
  if (root.scheme !== "file") return undefined;
  const segments = path.replaceAll("\\", "/").split("/");
  if (segments.length === 0 || segments.some(segment => segment.length === 0 || segment === "." || segment === "..")) return undefined;
  const rootPath = root.path.endsWith("/") ? root.path.slice(0, -1) : root.path;
  return root.withPath(`${rootPath}/${segments.map(encodeURIComponent).join("/")}`);
}

function fileKind(fileType: FsFileType): FileKind {
  switch (fileType) {
    case "directory":
      return FileKind.Directory;
    case "file":
      return FileKind.File;
    case "symbolicLink":
      return FileKind.SymbolicLink;
    case "other":
      return FileKind.Other;
  }
}
