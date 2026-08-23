import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { DocumentCollaborationConnection } from "../../../../editor/common/services/documentCollaborationService.js";
import type { DocumentCollaborationOpenInput } from "../../../../editor/common/services/documentCollaborationService.js";
import type { IDocumentCollaborationService } from "../../../../editor/common/services/documentCollaborationService.js";
import { RemoteDocumentCollaborationService } from "./remoteDocumentCollaborationService.js";

/** Routes Stanza collaboration to the local App Server or an explicit remote host. */
export class DocumentCollaborationService extends DisposableOwner implements IDocumentCollaborationService {
	private readonly remote = this.own(new RemoteDocumentCollaborationService());

	constructor(private readonly appServer: IDocumentCollaborationService | undefined) {
		super();
		if (appServer) this.own(appServer);
	}

	open(input: DocumentCollaborationOpenInput, signal: AbortSignal): Promise<DocumentCollaborationConnection> {
		if (input.target?.kind === "remote") return this.remote.open(input, signal);
		if (!this.appServer) return Promise.reject(new Error("This Stanza renderer has no local App Server collaboration transport"));
		return this.appServer.open(input, signal);
	}
}
