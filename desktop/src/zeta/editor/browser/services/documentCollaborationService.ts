import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { DocumentCollaborationConnection } from "../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationOpenInput } from "../../common/services/documentCollaborationService.js";
import type { IDocumentCollaborationService } from "../../common/services/documentCollaborationService.js";
import { RemoteDocumentCollaborationService } from "./remoteDocumentCollaborationService.js";

/** Routes Aster collaboration to the local App Server or an explicit remote host. */
export class DocumentCollaborationService extends DisposableOwner implements IDocumentCollaborationService {
  private readonly remote = this.own(new RemoteDocumentCollaborationService());

  constructor(private readonly appServer: IDocumentCollaborationService | undefined) {
    super();
    if (appServer) this.own(appServer);
  }

  open(input: DocumentCollaborationOpenInput, signal: AbortSignal): Promise<DocumentCollaborationConnection> {
    if (input.target?.kind === "remote") return this.remote.open(input, signal);
    if (!this.appServer) return Promise.reject(new Error("This Aster renderer has no local App Server collaboration transport"));
    return this.appServer.open(input, signal);
  }
}
