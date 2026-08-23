import { type DocumentTransaction } from "../../../common/model/documentTransaction.js";

/** One ordered client submission to a Stanza collaboration authority. */
export interface DocumentCollaborationEnvelope {
	readonly clientId: string;
	readonly sequence: number;
	readonly baseVersion: number;
	readonly transaction: DocumentTransaction;
}

/** A server-ordered collaboration submission with its committed document version. */
export interface DocumentCollaborationRemoteEnvelope extends DocumentCollaborationEnvelope {
	readonly version: number;
}

export type DocumentCollaborationAcknowledgement = DocumentCollaborationRemoteEnvelope;
