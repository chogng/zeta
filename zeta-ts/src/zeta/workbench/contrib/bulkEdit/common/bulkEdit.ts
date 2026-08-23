import { type IDisposable } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { type LanguageWorkspaceEdit } from "../../../../editor/common/languages/languageWorkspaceEdit.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import { type WorkspaceEditResult } from "../../../services/language/common/workspaceEditService.js";

/** Controls whether a caller wants to stop in the Workbench preview before applying. */
export type BulkEditPreviewMode = "never" | "always";

/** Options that make a multi-resource edit application explicit at the call site. */
export interface BulkEditApplyOptions {
  readonly preview?: BulkEditPreviewMode;
  readonly signal?: AbortSignal;
}

/** Result of a bulk edit, including whether the user actually accepted it. */
export interface BulkEditResult extends WorkspaceEditResult {
  readonly applied: boolean;
}

/** Workbench callback used by the preview contribution to filter an edit. */
export type BulkEditPreviewHandler = (edit: LanguageWorkspaceEdit, signal: AbortSignal) => Promise<LanguageWorkspaceEdit | undefined>;

/** Workbench owner for preview policy around the lower-level workspace edit transaction. */
export interface IBulkEditService {
  apply(edit: LanguageWorkspaceEdit, options?: BulkEditApplyOptions): Promise<BulkEditResult>;
  hasPreviewHandler(): boolean;
  setPreviewHandler(handler: BulkEditPreviewHandler): IDisposable;
}

export const IBulkEditService = createServiceIdentifier<IBulkEditService>("bulkEditService");

export type BulkEditPreviewEntryKind = "textDocument" | "create" | "rename" | "delete";

/** One selectable row in the Workbench bulk-edit preview. */
export interface BulkEditPreviewEntry {
  readonly index: number;
  readonly kind: BulkEditPreviewEntryKind;
  readonly resource: URI;
  readonly secondaryResource?: URI;
  readonly detail: string;
  readonly before?: string;
  readonly after?: string;
  readonly error?: string;
}

/** Materialized preview data; the original ordered edit remains the apply contract. */
export interface BulkEditPreviewModel {
  readonly edit: LanguageWorkspaceEdit;
  readonly entries: readonly BulkEditPreviewEntry[];
  readonly canApply: boolean;
}
