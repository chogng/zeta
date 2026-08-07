import assert from "node:assert/strict";
import test from "node:test";
import { RemoteDocumentCollaborationService } from "../../browser/services/remoteDocumentCollaborationService.js";
import type { DocumentNode } from "../../common/model/document.js";
import { createDefaultDocumentSchema, type DocumentSchema } from "../../common/model/documentSchema.js";
import { DocumentTransaction } from "../../common/model/documentTransaction.js";
import { serializeDocument } from "../../common/model/documentSerialization.js";
import { serializeDocumentTransaction } from "../../common/model/documentTransactionSerialization.js";

const TOKEN = "0123456789abcdef0123456789abcdef";

test("Gama remote collaboration uses authenticated HTTP without App Server or session state", async () => {
  const schema = createDefaultDocumentSchema();
  const document = createDocument(schema);
  const requests: Request[] = [];
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async (input, init) => {
    const request = new Request(input, init);
    requests.push(request);
    if (request.url.endsWith("/rooms/open")) {
      return jsonResponse({ clientId: "client-a", schemaId: "gama-v1", snapshot: { roomId: "gama-remote", version: 0, document: serializeDocument(document, schema) } });
    }
    if (request.url.endsWith("/rooms/submit")) {
      const transaction = new DocumentTransaction().replaceText("text-1", 0, 0, "A");
      return jsonResponse({ status: "accepted", update: { roomId: "gama-remote", clientId: "client-a", sequence: 1, baseVersion: 0, version: 1, transaction: serializeDocumentTransaction(transaction, schema) } });
    }
    return new Promise<Response>((_resolve, reject) => request.signal.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true }));
  }) as typeof fetch;
  try {
    using service = new RemoteDocumentCollaborationService();
    using connection = await service.open({ clientId: "client-a", schemaId: "gama-v1", schema, document, target: { kind: "remote", endpoint: "https://collaboration.zeta.example", bearerToken: TOKEN } }, new AbortController().signal);
    const transaction = new DocumentTransaction().replaceText("text-1", 0, 0, "A");
    const outcome = await connection.submit({ clientId: "client-a", sequence: 1, baseVersion: 0, transaction }, schema.createDocument([schema.createNode("paragraph", { id: "paragraph-1", content: [schema.createText("AHello", { id: "text-1" })] })], "document-1"), new AbortController().signal);

    assert.equal(connection.roomId, "gama-remote");
    assert.equal(outcome.kind, "accepted");
    assert.equal(outcome.kind === "accepted" ? outcome.update.version : -1, 1);
    assert.equal(requests[0]?.url, "https://collaboration.zeta.example/v1/document-collaboration/rooms/open");
    assert.equal(requests[0]?.headers.get("authorization"), `Bearer ${TOKEN}`);
    const submitRequest = requests.find(request => request.url.endsWith("/rooms/submit"));
    assert.equal(submitRequest?.url, "https://collaboration.zeta.example/v1/document-collaboration/rooms/submit");
    assert.equal(submitRequest?.headers.get("authorization"), `Bearer ${TOKEN}`);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("Gama remote collaboration delivers ordered long-poll updates to its connection", async () => {
  const schema = createDefaultDocumentSchema();
  const document = createDocument(schema);
  const remoteTransaction = new DocumentTransaction().replaceText("text-1", 0, 0, "R");
  let resolvePoll: (() => void) | undefined;
  const originalFetch = globalThis.fetch;
  globalThis.fetch = ((input, init) => {
    const request = new Request(input, init);
    if (request.url.endsWith("/rooms/open")) return Promise.resolve(jsonResponse({ clientId: "client-a", schemaId: "gama-v1", snapshot: { roomId: "gama-remote", version: 0, document: serializeDocument(document, schema) } }));
    return new Promise<Response>((resolve, reject) => {
      const onAbort = () => reject(new DOMException("Aborted", "AbortError"));
      request.signal.addEventListener("abort", onAbort, { once: true });
      resolvePoll = () => {
        request.signal.removeEventListener("abort", onAbort);
        resolve(jsonResponse({ status: "updates", updates: [{ roomId: "gama-remote", clientId: "client-b", sequence: 1, baseVersion: 0, version: 1, transaction: serializeDocumentTransaction(remoteTransaction, schema) }] }));
      };
    });
  }) as typeof fetch;
  try {
    using service = new RemoteDocumentCollaborationService();
    using connection = await service.open({ clientId: "client-a", schemaId: "gama-v1", schema, document, target: { kind: "remote", endpoint: "https://collaboration.zeta.example", bearerToken: TOKEN } }, new AbortController().signal);
    const updates: DocumentTransaction[] = [];
    connection.onDidReceiveUpdate(update => updates.push(update.transaction));
    await waitFor(() => resolvePoll !== undefined);
    resolvePoll?.();
    await waitFor(() => updates.length === 1);

    assert.deepEqual(updates[0]?.steps, remoteTransaction.steps);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("Gama remote collaboration retries a transient long-poll transport failure", async () => {
  const schema = createDefaultDocumentSchema();
  const document = createDocument(schema);
  const remoteTransaction = new DocumentTransaction().replaceText("text-1", 0, 0, "R");
  let polls = 0;
  const originalFetch = globalThis.fetch;
  globalThis.fetch = ((input, init) => {
    const request = new Request(input, init);
    if (request.url.endsWith("/rooms/open")) return Promise.resolve(jsonResponse({ clientId: "client-a", schemaId: "gama-v1", snapshot: { roomId: "gama-remote", version: 0, document: serializeDocument(document, schema) } }));
    polls += 1;
    if (polls === 1) return Promise.reject(new TypeError("temporary network failure"));
    if (polls === 2) return Promise.resolve(jsonResponse({ status: "updates", updates: [{ roomId: "gama-remote", clientId: "client-b", sequence: 1, baseVersion: 0, version: 1, transaction: serializeDocumentTransaction(remoteTransaction, schema) }] }));
    return new Promise<Response>((_resolve, reject) => request.signal.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true }));
  }) as typeof fetch;
  try {
    using service = new RemoteDocumentCollaborationService();
    using connection = await service.open({ clientId: "client-a", schemaId: "gama-v1", schema, document, target: { kind: "remote", endpoint: "https://collaboration.zeta.example", bearerToken: TOKEN } }, new AbortController().signal);
    const updates: DocumentTransaction[] = [];
    connection.onDidReceiveUpdate(update => updates.push(update.transaction));
    await new Promise(resolve => setTimeout(resolve, 350));

    assert.equal(polls >= 2, true);
    assert.deepEqual(updates[0]?.steps, remoteTransaction.steps);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

function createDocument(schema: DocumentSchema): DocumentNode {
  return schema.createDocument([schema.createNode("paragraph", { id: "paragraph-1", content: [schema.createText("Hello", { id: "text-1" })] })], "document-1");
}

function jsonResponse(value: object): Response {
  return new Response(JSON.stringify(value), { status: 200, headers: { "Content-Type": "application/json" } });
}

async function waitFor(condition: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (condition()) return;
    await Promise.resolve();
  }
  assert.fail("Expected asynchronous collaboration condition to become true");
}
