import type { IDocumentCollaborationApi } from "../common/documentCollaborationApi.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";

export function createDisconnectedDocumentCollaborationApi(unavailable: UnavailableOperation): IDocumentCollaborationApi {
  return {
    open: () => unavailable("documentCollaboration.open"),
    submit: () => unavailable("documentCollaboration.submit"),
    publishPresence: () => unavailable("documentCollaboration.publishPresence"),
    readPresence: () => unavailable("documentCollaboration.readPresence"),
  };
}

export function createViteDevDocumentCollaborationApi(connection: ViteDevAppServerConnection): IDocumentCollaborationApi {
  return {
    open: params => viteDevRequest(connection, "document/collaboration/open", params),
    submit: params => viteDevRequest(connection, "document/collaboration/submit", params),
    publishPresence: params => viteDevRequest(connection, "document/collaboration/presence/publish", params),
    readPresence: params => viteDevRequest(connection, "document/collaboration/presence/read", params),
  };
}
