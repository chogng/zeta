import type { URI } from "../../../../base/common/uri.js";
import type { TextRange } from "../../../../editor/common/core/text.js";

/**
 * A resource requested by the Workbench editor host.
 *
 * Editors may use `contentType` as a matching hint, but the resource remains
 * the durable identity used when inputs move between panes.
 */
export interface EditorInput {
  readonly resource: URI;
  readonly contentType?: string;
  /** Resolved only by the editor host for dynamic language-package matching. */
  readonly languageId?: string;
  readonly label?: string;
  /** Requests a non-mutating editor instance while preserving selection and navigation. */
  readonly readOnly?: boolean;
  /**
   * Optional in-memory bootstrap text used until the document service owns
   * loading and saving. Editor panes must treat this as an initial snapshot,
   * not as durable storage.
   */
  readonly initialText?: string;
}

/** Optional caller preference used by "Open With" and saved associations. */
export interface EditorOpenOptions {
  readonly preferredEditorId?: string;
  /** Inserts or moves the editor to this zero-based tab index. */
  readonly index?: number;
  /** Selects and reveals this range after the target pane becomes active. */
  readonly selection?: TextRange;
}
