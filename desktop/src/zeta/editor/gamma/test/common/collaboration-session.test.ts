import assert from "node:assert/strict";
import test from "node:test";
import { DocumentSerializationError } from "../../common/serialization.js";
import { createDefaultDocumentSchema, type DocumentSchema } from "../../common/schema.js";
import { textSelection } from "../../common/selection.js";
import { DocumentTransaction } from "../../common/transaction.js";
import { deserializeDocumentCollaborationEnvelope, serializeDocumentCollaborationEnvelope } from "../../contrib/collaboration/common/envelopeSerialization.js";
import { DocumentCollaborationSession, DocumentCollaborationError } from "../../contrib/collaboration/common/session.js";

function createDocument(schema: DocumentSchema) {
  return schema.createDocument([schema.createNode("paragraph", { id: "paragraph-1", content: [schema.createText("Hello", { id: "text-1" })] })], "document-1");
}

test("Gamma collaboration session keeps canonical and optimistic snapshots separate", () => {
  const schema = createDefaultDocumentSchema();
  using session = new DocumentCollaborationSession({ schema, document: createDocument(schema), clientId: "client-a", version: 1 });
  const kinds: string[] = [];
  session.onDidChange(change => kinds.push(change.kind));

  const local = session.dispatchLocal(new DocumentTransaction().replaceText("text-1", 1, 1, "L"));
  assert.equal(local?.sequence, 1);
  assert.equal(local?.baseVersion, 1);
  assert.equal(session.canonicalDocument.content[0]?.content[0]?.text, "Hello");
  assert.equal(session.document.content[0]?.content[0]?.text, "HLello");
  assert.ok(session.pending);

  const remote = session.receiveRemote({
    clientId: "client-b",
    sequence: 4,
    baseVersion: 1,
    version: 2,
    transaction: new DocumentTransaction().replaceText("text-1", 0, 0, "R"),
  });
  assert.equal(remote.kind, "remote");
  assert.equal(remote.canonicalDocument.content[0]?.content[0]?.text, "RHello");
  assert.equal(remote.document.content[0]?.content[0]?.text, "RHLello");
  assert.equal(remote.pending?.steps[0]?.kind, "replaceText");
  assert.equal(remote.pending?.steps[0]?.kind === "replaceText" ? remote.pending.steps[0].from : -1, 2);
  assert.equal(session.version, 2);

  const acknowledgement = session.acknowledge({
    clientId: "client-a",
    sequence: 1,
    baseVersion: 2,
    version: 3,
    transaction: remote.pending!,
  });
  assert.equal(acknowledgement.kind, "acknowledged");
  assert.equal(session.document.content[0]?.content[0]?.text, "RHLello");
  assert.equal(session.canonicalDocument.content[0]?.content[0]?.text, "RHLello");
  assert.equal(session.pending, undefined);
  assert.equal(session.version, 3);
  assert.deepEqual(kinds, ["local", "remote", "acknowledged"]);
});

test("Gamma collaboration session emits cumulative pending updates", () => {
  const schema = createDefaultDocumentSchema();
  using session = new DocumentCollaborationSession({ schema, document: createDocument(schema), clientId: "client-a" });

  const first = session.dispatchLocal(new DocumentTransaction().replaceText("text-1", 0, 0, "A"));
  const second = session.dispatchLocal(new DocumentTransaction().replaceText("text-1", 1, 1, "B"));

  assert.equal(first?.sequence, 1);
  assert.equal(second?.sequence, 2);
  assert.equal(second?.transaction.steps.length, 2);
  assert.equal(session.document.content[0]?.content[0]?.text, "ABHello");
  assert.equal(session.pendingSequence, 2);
});

test("Gamma collaboration envelopes round-trip local and remote transaction versions", () => {
  const schema = createDefaultDocumentSchema();
  using session = new DocumentCollaborationSession({ schema, document: createDocument(schema), clientId: "client-a", version: 3 });
  const local = session.dispatchLocal(new DocumentTransaction().replaceText("text-1", 0, 0, "L"));
  assert.ok(local);

  const decodedLocal = deserializeDocumentCollaborationEnvelope(serializeDocumentCollaborationEnvelope(local, schema), schema);
  assert.equal("documentVersion" in decodedLocal, false);
  assert.deepEqual(decodedLocal.transaction.steps, local.transaction.steps);
  assert.equal(decodedLocal.baseVersion, 3);

  const remote = { clientId: "client-b", sequence: 7, baseVersion: 3, version: 4, transaction: new DocumentTransaction().replaceText("text-1", 0, 0, "R") } as const;
  const decodedRemote = deserializeDocumentCollaborationEnvelope(serializeDocumentCollaborationEnvelope(remote, schema), schema);
  assert.equal("version" in decodedRemote ? decodedRemote.version : -1, 4);
  assert.deepEqual(decodedRemote.transaction.steps, remote.transaction.steps);
  assert.throws(() => deserializeDocumentCollaborationEnvelope("{\"format\":\"zeta.document.collaboration\",\"version\":99}", schema), DocumentSerializationError);
});

test("Gamma collaboration session rejects stale updates and local echoes", () => {
  const schema = createDefaultDocumentSchema();
  using session = new DocumentCollaborationSession({ schema, document: createDocument(schema), clientId: "client-a", version: 4 });

  assert.throws(() => session.receiveRemote({ clientId: "client-b", sequence: 1, baseVersion: 3, version: 5, transaction: new DocumentTransaction().replaceText("text-1", 0, 0, "R") }), DocumentCollaborationError);
  assert.throws(() => session.receiveRemote({ clientId: "client-a", sequence: 1, baseVersion: 4, version: 5, transaction: new DocumentTransaction().replaceText("text-1", 0, 0, "R") }), DocumentCollaborationError);
  assert.equal(session.dispatchLocal(new DocumentTransaction().withSelection(textSelection({ nodeId: "text-1", offset: 2 }))), undefined);
  assert.equal(session.document.content[0]?.content[0]?.text, "Hello");
});

test("Gamma collaboration session reports dropped pending steps after remote deletion", () => {
  const schema = createDefaultDocumentSchema();
  const document = schema.createDocument([
    schema.createNode("paragraph", { id: "paragraph-1", content: [schema.createText("One", { id: "text-1" })] }),
    schema.createNode("paragraph", { id: "paragraph-2", content: [schema.createText("Two", { id: "text-2" })] }),
  ], "document-1");
  using session = new DocumentCollaborationSession({ schema, document, clientId: "client-a" });
  session.dispatchLocal(new DocumentTransaction().setNodeAttributes("paragraph-2", { alignment: "center" }));

  const change = session.receiveRemote({ clientId: "client-b", sequence: 1, baseVersion: 1, version: 2, transaction: new DocumentTransaction().deleteNode("paragraph-2") });

  assert.equal(change.droppedSteps.length, 1);
  assert.equal(session.pending, undefined);
  assert.deepEqual(session.document.content.map(node => node.id), ["paragraph-1"]);
});
