import type {
  FsFileType,
  FsGetMetadataParams,
  FsGetMetadataResult,
  FsReadDirectoryParams,
  FsReadDirectoryResult,
} from "../../../../../generated/app-server/types.js";
import { URI } from "../../../base/common/uri.js";
import {
  FileKind,
  type IFileEntry,
  type IFileService,
  type IFileStat,
} from "../common/files.js";
import type {
  IWorkspaceContextService,
} from "../../workspace/common/workspace.js";

/** Narrow App Server surface consumed by the browser file-service adapter. */
export interface IFileSystemApi {
  getMetadata(params: FsGetMetadataParams): Promise<FsGetMetadataResult>;
  readDirectory(params: FsReadDirectoryParams): Promise<FsReadDirectoryResult>;
}

export interface BrowserFileServiceOptions {
  readonly api: IFileSystemApi;
  readonly workspaceContextService: IWorkspaceContextService;
}

/**
 * Maps workspace resource URIs to the App Server's root-relative filesystem protocol.
 */
export class BrowserFileService implements IFileService {
  readonly #api: IFileSystemApi;
  readonly #workspaceContextService: IWorkspaceContextService;

  constructor(options: BrowserFileServiceOptions) {
    this.#api = options.api;
    this.#workspaceContextService = options.workspaceContextService;
  }

  async stat(resource: URI): Promise<IFileStat> {
    const result = await this.#api.getMetadata({
      path: this.#relativePath(resource),
    });
    return {
      resource,
      kind: fileKind(result.fileType),
      sizeBytes: result.sizeBytes,
      readonly: result.readonly,
      modifiedAtMillis: result.modifiedAtMillis ?? undefined,
    };
  }

  async readDirectory(resource: URI): Promise<readonly IFileEntry[]> {
    const result = await this.#api.readDirectory({
      path: this.#relativePath(resource),
    });
    return result.entries.map((entry) => ({
      resource: childResource(resource, entry.name),
      name: entry.name,
      kind: fileKind(entry.fileType),
    }));
  }

  #relativePath(resource: URI): string {
    const folders = this.#workspaceContextService.getWorkspace().folders;
    if (folders.length !== 1) {
      throw new Error(
        "The current filesystem protocol requires one workspace folder",
      );
    }
    return workspaceRelativePath(folders[0].uri, resource);
  }
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
  if (comparedResource === comparedRoot) return "";
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
