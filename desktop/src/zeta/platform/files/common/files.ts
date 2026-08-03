import type { Event } from "../../../base/common/event.js";
import type { URI } from "../../../base/common/uri.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

/** Stable file kind used by Workbench consumers independently of wire DTOs. */
export enum FileKind {
  Directory = "directory",
  File = "file",
  SymbolicLink = "symbolicLink",
  Other = "other",
}

/** Metadata for one existing workspace resource. */
export interface IFileStat {
  readonly resource: URI;
  readonly kind: FileKind;
  readonly sizeBytes: number;
  readonly readonly: boolean;
  readonly modifiedAtMillis: number | undefined;
}

/** One direct child returned by a directory read. */
export interface IFileEntry {
  readonly resource: URI;
  readonly name: string;
  readonly kind: FileKind;
}

/** Coarse invalidation reported after workspace files may have changed on disk. */
export interface IFileChangeEvent {
  readonly resources: readonly URI[] | undefined;
}

/** Workspace-scoped file operations available to Workbench features. */
export interface IFileService {
  readonly onDidChangeFiles: Event<IFileChangeEvent>;
  stat(resource: URI): Promise<IFileStat>;
  readDirectory(resource: URI): Promise<readonly IFileEntry[]>;
  readFile(resource: URI): Promise<string>;
  writeFile(resource: URI, content: string): Promise<IFileStat>;
}

export const IFileService =
  createServiceIdentifier<IFileService>("fileService");
