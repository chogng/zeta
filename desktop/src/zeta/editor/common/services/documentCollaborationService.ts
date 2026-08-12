import type { Event } from "../../../base/common/event.js";
import type { IDisposable } from "../../../base/common/lifecycle.js";
import type { DocumentNode } from "../model/document.js";
import type { DocumentSchema } from "../model/documentSchema.js";
import type { DocumentSelection } from "../core/documentSelection.js";
import type { DocumentCollaborationEnvelope } from "../../contrib/collaboration/common/protocol.js";
import type { DocumentCollaborationRemoteEnvelope } from "../../contrib/collaboration/common/protocol.js";

/** Selects an editor-local App Server room or one explicitly configured remote host. */
export type DocumentCollaborationTarget =
  | { readonly kind: "appServer" }
  | { readonly kind: "remote"; readonly endpoint: string; readonly bearerToken: string };

/** Inputs needed to create or join one server-ordered Aster collaboration room. */
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

/** One other collaborator's current ephemeral selection. */
export interface DocumentCollaborationPresence {
  readonly clientId: string;
  readonly selection: DocumentSelection;
}

/** Role assigned to a member invited into an authenticated remote room. */
export type DocumentCollaborationRoomRole = "owner" | "editor" | "viewer";

/** A newly issued room credential, exposed once to the inviting owner. */
export interface DocumentCollaborationInvite {
  readonly roomId: string;
  readonly principalId: string;
  readonly displayName: string;
  readonly role: DocumentCollaborationRoomRole;
  readonly accessToken: string;
}

/** One active remote collaboration member visible to an authenticated room owner. */
export interface DocumentCollaborationMember {
  readonly principalId: string;
  readonly displayName: string;
  readonly role: DocumentCollaborationRoomRole;
}

export type DocumentCollaborationSubmitOutcome =
  | { readonly kind: "accepted"; readonly update: DocumentCollaborationRemoteEnvelope }
  | { readonly kind: "conflict"; readonly updates: readonly DocumentCollaborationRemoteEnvelope[] }
  | { readonly kind: "resync"; readonly snapshot: DocumentCollaborationSnapshot };

/** One lifetime-bound room connection independent of browser or App Server transports. */
export interface DocumentCollaborationConnection extends IDisposable {
  readonly roomId: string;
  readonly clientId: string;
  /** Persistent member identity for authenticated remote rooms; absent for local App Server rooms. */
  readonly principalId: string | undefined;
  /** Whether this authenticated room connection may create document updates. */
  readonly canEdit: boolean;
  /** Whether this authenticated room connection may create member credentials. */
  readonly canManageMembers: boolean;
  readonly schema: DocumentSchema;
  readonly initialSnapshot: DocumentCollaborationSnapshot;
  /** Current remote selections known at connection creation or from later transport events. */
  readonly currentPresence: readonly DocumentCollaborationPresence[];
  readonly onDidReceiveUpdate: Event<DocumentCollaborationRemoteEnvelope>;
  readonly onDidReceiveSnapshot: Event<DocumentCollaborationSnapshot>;
  readonly onDidReceivePresence: Event<readonly DocumentCollaborationPresence[]>;
  readonly onDidFail: Event<Error>;
  submit(envelope: DocumentCollaborationEnvelope, document: DocumentNode, signal: AbortSignal): Promise<DocumentCollaborationSubmitOutcome>;
  updatePresence(selection: DocumentSelection | undefined, signal: AbortSignal): Promise<void>;
  createInvite(displayName: string, role: DocumentCollaborationRoomRole, signal: AbortSignal): Promise<DocumentCollaborationInvite>;
  listMembers(signal: AbortSignal): Promise<readonly DocumentCollaborationMember[]>;
  rotateMemberAccessToken(principalId: string, signal: AbortSignal): Promise<DocumentCollaborationInvite>;
  revokeMember(principalId: string, signal: AbortSignal): Promise<void>;
}

/** Opens Aster collaboration rooms and turns transport payloads into schema-valid domain values. */
export interface IDocumentCollaborationService extends IDisposable {
  open(input: DocumentCollaborationOpenInput, signal: AbortSignal): Promise<DocumentCollaborationConnection>;
}
