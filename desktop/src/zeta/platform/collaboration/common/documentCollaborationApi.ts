import type { DocumentCollaborationOpenParams } from "../../../../../generated/app-server/types.js";
import type { DocumentCollaborationOpenResult } from "../../../../../generated/app-server/types.js";
import type { DocumentCollaborationSubmitParams } from "../../../../../generated/app-server/types.js";
import type { DocumentCollaborationSubmitResult } from "../../../../../generated/app-server/types.js";

/** Typed transport boundary for server-ordered structured-document collaboration. */
export interface IDocumentCollaborationApi {
  open(params: DocumentCollaborationOpenParams): Promise<DocumentCollaborationOpenResult>;
  submit(params: DocumentCollaborationSubmitParams): Promise<DocumentCollaborationSubmitResult>;
}
