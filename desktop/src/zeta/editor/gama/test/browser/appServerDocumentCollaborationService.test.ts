import assert from "node:assert/strict";
import test from "node:test";
import type { DocumentCollaborationOpenParams } from "../../../../../../generated/app-server/types.js";
import type { DocumentCollaborationOpenResult } from "../../../../../../generated/app-server/types.js";
import type { DocumentCollaborationSubmitParams } from "../../../../../../generated/app-server/types.js";
import type { DocumentCollaborationSubmitResult } from "../../../../../../generated/app-server/types.js";
import type { DocumentCollaborationUpdate } from "../../../../../../generated/app-server/types.js";
import type { ServerNotification } from "../../../../../../generated/app-server/types.js";
import type { IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { IDocumentCollaborationApi } from "../../../../platform/collaboration/common/documentCollaborationApi.js";
import { AppServerDocumentCollaborationService } from "../../browser/services/appServerDocumentCollaborationService.js";
import type { DocumentNode } from "../../common/model/document.js";
import { createDefaultDocumentSchema, type DocumentSchema } from "../../common/model/documentSchema.js";
import { applyDocumentTransaction, DocumentTransaction } from "../../common/model/documentTransaction.js";
import { serializeDocumentTransaction } from "../../common/model/documentTransactionSerialization.js";
import type { DocumentCollaborationRemoteEnvelope } from "../../contrib/collaboration/common/session.js";

test("Gama App Server collaboration adapter uses JSON-safe ordered versions and delivers room updates", async () => {
  const schema = createDefaultDocumentSchema();
  const document = createDocument(schema);
  const events = new FakeServerEvents();
  const api = new FakeDocumentCollaborationApi();
  using service = new AppServerDocumentCollaborationService(api, events);
  using connection = await service.open({ clientId: "client-a", schemaId: "gama-default-v1", schema, document }, new AbortController().signal);

  assert.equal(connection.roomId, "room-a");
  assert.equal(connection.initialSnapshot.version, 0);
  const received: DocumentCollaborationRemoteEnvelope[] = [];
  connection.onDidReceiveUpdate(update => received.push(update));
  const remote = new DocumentTransaction().replaceText("text-1", 0, 0, "R");
  events.fire({ roomId: "room-a", clientId: "client-b", sequence: 1, baseVersion: 0, version: 1, transaction: serializeDocumentTransaction(remote, schema) });
  assert.equal(received.length, 1);
  assert.equal(received[0]?.version, 1);
  assert.deepEqual(received[0]?.transaction.steps, remote.steps);

  const local = new DocumentTransaction().replaceText("text-1", 0, 0, "A");
  const localDocument = applyDocumentTransaction(document, schema, local).document;
  const outcome = await connection.submit({ clientId: "client-a", sequence: 1, baseVersion: 0, transaction: local }, localDocument, new AbortController().signal);
  assert.equal(api.submissions[0]?.sequence, 1);
  assert.equal(api.submissions[0]?.baseVersion, 0);
  assert.equal(outcome.kind, "accepted");
  assert.equal(outcome.kind === "accepted" ? outcome.update.version : -1, 1);
});

function createDocument(schema: DocumentSchema): DocumentNode {
  return schema.createDocument([schema.createNode("paragraph", { id: "paragraph-1", content: [schema.createText("Hello", { id: "text-1" })] })], "document-1");
}

class FakeDocumentCollaborationApi implements IDocumentCollaborationApi {
  readonly submissions: DocumentCollaborationSubmitParams[] = [];

  async open(params: DocumentCollaborationOpenParams): Promise<DocumentCollaborationOpenResult> {
    return { clientId: params.clientId, schemaId: params.schemaId, snapshot: { roomId: "room-a", version: 0, document: params.document } };
  }

  async submit(params: DocumentCollaborationSubmitParams): Promise<DocumentCollaborationSubmitResult> {
    this.submissions.push(params);
    return {
      status: "accepted",
      update: {
        roomId: params.roomId,
        clientId: params.clientId,
        sequence: params.sequence,
        baseVersion: params.baseVersion,
        version: params.baseVersion + 1,
        transaction: params.transaction,
      },
    };
  }
}

class FakeServerEvents implements IServerEventApi {
  private readonly listeners = new Set<(event: ServerNotification) => void>();

  subscribe(listener: (event: ServerNotification) => void): { dispose(): void } {
    this.listeners.add(listener);
    return { dispose: () => this.listeners.delete(listener) };
  }

  fire(update: DocumentCollaborationUpdate): void {
    const event: ServerNotification = { method: "document/collaboration/update", params: update };
    for (const listener of this.listeners) listener(event);
  }
}
