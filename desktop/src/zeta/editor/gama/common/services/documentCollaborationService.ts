import type { Event } from "../../../../base/common/event.js";
import type { IDisposable } from "../../../../base/common/lifecycle.js";
import type { DocumentNode } from "../model/document.js";
import type { DocumentSchema } from "../model/documentSchema.js";
import type { DocumentCollaborationEnvelope } from "../../contrib/collaboration/common/session.js";
import type { DocumentCollaborationRemoteEnvelope } from "../../contrib/collaboration/common/session.js";

/** Selects an editor-local App Server room or one explicitly configured remote host. */
export type DocumentCollaborationTarget =
  | { readonly kind: "appServer" }
  | { readonly kind: "remote"; readonly endpoint: string; readonly bearerToken: string };

/** Inputs needed to create or join one server-ordered Gama collaboration room. */
export interface DocumentCollaborationOpenInput {
  readonly roomId?: string;
  readonly clientId: string;
  readonly schemaId: string;
  readonly schema: DocumentSchema;
  readonly document: DocumentNode;
  /** Omitting a target preserves the App Server transport used by existing hosts. */
  readonly target?: DocumentCollaborationTarget;
}

/** Canonical room snapshot supplied after joining or resynchronizing. */
export interface DocumentCollaborationSnapshot {
  readonly roomId: string;
  readonly version: number;
  readonly document: DocumentNode;
}

export type DocumentCollaborationSubmitOutcome =
  | { readonly kind: "accepted"; readonly update: DocumentCollaborationRemoteEnvelope }
  | { readonly kind: "conflict"; readonly updates: readonly DocumentCollaborationRemoteEnvelope[] }
  | { readonly kind: "resync"; readonly snapshot: DocumentCollaborationSnapshot };

/** One lifetime-bound room connection independent of browser or App Server transports. */
export interface DocumentCollaborationConnection extends IDisposable {
  readonly roomId: string;
  readonly clientId: string;
  readonly schema: DocumentSchema;
  readonly initialSnapshot: DocumentCollaborationSnapshot;
  readonly onDidReceiveUpdate: Event<DocumentCollaborationRemoteEnvelope>;
  readonly onDidReceiveSnapshot: Event<DocumentCollaborationSnapshot>;
  readonly onDidFail: Event<Error>;
  submit(envelope: DocumentCollaborationEnvelope, document: DocumentNode, signal: AbortSignal): Promise<DocumentCollaborationSubmitOutcome>;
}

/** Opens Gama collaboration rooms and turns transport payloads into schema-valid domain values. */
export interface IDocumentCollaborationService extends IDisposable {
  open(input: DocumentCollaborationOpenInput, signal: AbortSignal): Promise<DocumentCollaborationConnection>;
}
