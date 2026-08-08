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
    if (request.url.includes("/presence?")) return new Promise<Response>((_resolve, reject) => request.signal.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true }));
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
    if (request.url.includes("/presence?")) return new Promise<Response>((_resolve, reject) => request.signal.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true }));
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

test("Gama remote collaboration publishes local presence and projects remote selections", async () => {
  const schema = createDefaultDocumentSchema();
  const document = createDocument(schema);
  let resolvePresence: (() => void) | undefined;
  const requests: Request[] = [];
  const originalFetch = globalThis.fetch;
  globalThis.fetch = ((input, init) => {
    const request = new Request(input, init);
    requests.push(request);
    if (request.url.endsWith("/rooms/open")) return Promise.resolve(jsonResponse({ clientId: "client-a", schemaId: "gama-v1", snapshot: { roomId: "gama-remote", version: 0, document: serializeDocument(document, schema) } }));
    if (request.url.endsWith("/rooms/presence")) return Promise.resolve(new Response(null, { status: 204 }));
    if (request.url.includes("/presence?")) {
      return new Promise<Response>((resolve, reject) => {
        const onAbort = () => reject(new DOMException("Aborted", "AbortError"));
        request.signal.addEventListener("abort", onAbort, { once: true });
        resolvePresence = () => {
          request.signal.removeEventListener("abort", onAbort);
          resolve(jsonResponse({ generation: 1, presences: [{ clientId: "client-b", selection: JSON.stringify({ kind: "text", anchor: { nodeId: "text-1", offset: 0 }, head: { nodeId: "text-1", offset: 1 } }) }] }));
        };
      });
    }
    return new Promise<Response>((_resolve, reject) => request.signal.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true }));
  }) as typeof fetch;
  try {
    using service = new RemoteDocumentCollaborationService();
    using connection = await service.open({ clientId: "client-a", schemaId: "gama-v1", schema, document, target: { kind: "remote", endpoint: "https://collaboration.zeta.example", bearerToken: TOKEN } }, new AbortController().signal);
    const presences: (readonly { readonly clientId: string }[])[] = [];
    connection.onDidReceivePresence(presence => presences.push(presence));
    await connection.updatePresence({ kind: "text", anchor: { nodeId: "text-1", offset: 1 }, head: { nodeId: "text-1", offset: 1 } }, new AbortController().signal);
    await waitFor(() => resolvePresence !== undefined);
    resolvePresence?.();
    await waitFor(() => presences.length === 1);

    const publish = requests.find(request => request.url.endsWith("/rooms/presence"));
    assert.deepEqual(await publish?.json(), { roomId: "gama-remote", clientId: "client-a", selection: JSON.stringify({ kind: "text", anchor: { nodeId: "text-1", offset: 1 }, head: { nodeId: "text-1", offset: 1 } }) });
    assert.equal(presences[0]?.[0]?.clientId, "client-b");
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("Gama remote collaboration exposes a viewer room as read-only", async () => {
  const schema = createDefaultDocumentSchema();
  const document = createDocument(schema);
  const originalFetch = globalThis.fetch;
  globalThis.fetch = ((input, init) => {
    const request = new Request(input, init);
    if (request.url.endsWith("/rooms/open")) return Promise.resolve(jsonResponse({ clientId: "client-viewer", schemaId: "gama-v1", snapshot: { roomId: "gama-remote", version: 0, document: serializeDocument(document, schema) }, canEdit: false, canManageMembers: false }));
    return new Promise<Response>((_resolve, reject) => request.signal.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true }));
  }) as typeof fetch;
  try {
    using service = new RemoteDocumentCollaborationService();
    using connection = await service.open({ clientId: "client-viewer", schemaId: "gama-v1", schema, document, target: { kind: "remote", endpoint: "https://collaboration.zeta.example", bearerToken: TOKEN } }, new AbortController().signal);
    assert.equal(connection.canEdit, false);
    assert.equal(connection.canManageMembers, false);
    await assert.rejects(connection.createInvite("Writer", "editor", new AbortController().signal), /cannot create room invitations/);
    await assert.rejects(connection.listMembers(new AbortController().signal), /cannot inspect room members/);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("Gama remote collaboration owners create typed room invitations", async () => {
  const schema = createDefaultDocumentSchema();
  const document = createDocument(schema);
  const requests: Request[] = [];
  const originalFetch = globalThis.fetch;
  globalThis.fetch = ((input, init) => {
    const request = new Request(input, init);
    requests.push(request);
    if (request.url.endsWith("/rooms/open")) return Promise.resolve(jsonResponse({ clientId: "client-owner", principalId: "owner-1", schemaId: "gama-v1", snapshot: { roomId: "gama-remote", version: 0, document: serializeDocument(document, schema) }, canEdit: true, canManageMembers: true }));
    if (request.url.endsWith("/rooms/invites")) return Promise.resolve(jsonResponse({ roomId: "gama-remote", principalId: "member-1", displayName: "Writer", role: "editor", accessToken: "member-token" }));
    if (request.url.endsWith("/gama-remote/members")) return Promise.resolve(jsonResponse({ members: [{ principalId: "owner-1", displayName: "Owner", role: "owner" }, { principalId: "member-1", displayName: "Writer", role: "editor" }] }));
    if (request.url.endsWith("/rooms/members/rotate-token")) return Promise.resolve(jsonResponse({ roomId: "gama-remote", principalId: "member-1", displayName: "Writer", role: "editor", accessToken: "rotated-token" }));
    if (request.url.endsWith("/rooms/members/revoke")) return Promise.resolve(new Response(null, { status: 204 }));
    return new Promise<Response>((_resolve, reject) => request.signal.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true }));
  }) as typeof fetch;
  try {
    using service = new RemoteDocumentCollaborationService();
    using connection = await service.open({ clientId: "client-owner", schemaId: "gama-v1", schema, document, target: { kind: "remote", endpoint: "https://collaboration.zeta.example", bearerToken: TOKEN } }, new AbortController().signal);
    const invite = await connection.createInvite(" Writer ", "editor", new AbortController().signal);
    const members = await connection.listMembers(new AbortController().signal);
    const rotated = await connection.rotateMemberAccessToken("member-1", new AbortController().signal);
    await connection.revokeMember("member-1", new AbortController().signal);

    assert.equal(connection.canManageMembers, true);
    assert.equal(connection.principalId, "owner-1");
    assert.deepEqual(invite, { roomId: "gama-remote", principalId: "member-1", displayName: "Writer", role: "editor", accessToken: "member-token" });
    assert.deepEqual(members, [{ principalId: "owner-1", displayName: "Owner", role: "owner" }, { principalId: "member-1", displayName: "Writer", role: "editor" }]);
    assert.equal(rotated.accessToken, "rotated-token");
    const request = requests.find(candidate => candidate.url.endsWith("/rooms/invites"));
    assert.equal(request?.headers.get("authorization"), `Bearer ${TOKEN}`);
    assert.deepEqual(await request?.json(), { roomId: "gama-remote", displayName: "Writer", role: "editor" });
    const rotateRequest = requests.find(candidate => candidate.url.endsWith("/rooms/members/rotate-token"));
    assert.deepEqual(await rotateRequest?.json(), { roomId: "gama-remote", principalId: "member-1" });
    const revokeRequest = requests.find(candidate => candidate.url.endsWith("/rooms/members/revoke"));
    assert.deepEqual(await revokeRequest?.json(), { roomId: "gama-remote", principalId: "member-1" });
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
