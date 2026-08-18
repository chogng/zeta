import type { URI } from "../../../../base/common/uri.js";
import type { TextRange } from "../../../../editor/common/core/text.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

/** A resource requested through the Workbench editor service. */
export interface EditorInput {
  readonly resource: URI;
  readonly contentType?: string;
  readonly languageId?: string;
  readonly label?: string;
  readonly readOnly?: boolean;
  readonly initialText?: string;
}

/** Optional caller preferences for opening and revealing an editor resource. */
export interface EditorOpenOptions {
  readonly preferredEditorId?: string;
  readonly index?: number;
  readonly selection?: TextRange;
  /** Keeps the editor as a durable tab; `false` opens or replaces the group's preview tab. */
  readonly pinned?: boolean;
}

/** Resource-oriented editor operations available to Workbench contributions. */
export interface IEditorService {
  openEditor(input: EditorInput, options?: EditorOpenOptions): Promise<void>;
  focusActiveEditor(): void;
}

export const IEditorService = createServiceIdentifier<IEditorService>("editorService");
