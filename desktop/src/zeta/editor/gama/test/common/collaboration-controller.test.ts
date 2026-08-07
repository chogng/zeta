import assert from "node:assert/strict";
import test from "node:test";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { DocumentModel } from "../../common/model/documentModel.js";
import type { DocumentNode } from "../../common/model/document.js";
import { createDefaultDocumentSchema, type DocumentSchema } from "../../common/model/documentSchema.js";
import { DocumentTransaction } from "../../common/model/documentTransaction.js";
import type { DocumentCollaborationConnection } from "../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationSnapshot } from "../../common/services/documentCollaborationService.js";
import type { DocumentCollaborationSubmitOutcome } from "../../common/services/documentCollaborationService.js";
import { DocumentCollaborationController } from "../../contrib/collaboration/common/controller.js";
import type { DocumentCollaborationEnvelope } from "../../contrib/collaboration/common/session.js";
import type { DocumentCollaborationRemoteEnvelope } from "../../contrib/collaboration/common/session.js";

test("Gama collaboration submits only the in-flight snapshot while later typing is buffered", async () => {
  const schema = createDefaultDocumentSchema();
  const document = createDocument(schema);
  using model = new DocumentModel(schema, document);
  const connection = new FakeDocumentCollaborationConnection(schema, document);
  using controller = new DocumentCollaborationController(model, connection);

  model.dispatch(new DocumentTransaction().replaceText("text-1", 0, 0, "A"));
  assert.equal(connection.submissions.length, 1);
  assert.equal(firstText(connection.submissions[0]!.document), "AHello");

  model.dispatch(new DocumentTransaction().replaceText("text-1", 1, 1, "B"));
  assert.equal(firstText(model.document), "ABHello");
  assert.equal(connection.submissions.length, 1);

  connection.accept(0, 1);
  await flushMicrotasks();
  assert.equal(connection.submissions.length, 2);
  assert.equal(connection.submissions[1]!.envelope.sequence, 2);
  assert.equal(connection.submissions[1]!.envelope.baseVersion, 1);
  assert.equal(firstText(connection.submissions[1]!.document), "ABHello");

  connection.accept(1, 2);
  await flushMicrotasks();
  assert.equal(firstText(model.document), "ABHello");
  assert.equal(controller.state, "connected");
});

function createDocument(schema: DocumentSchema): DocumentNode {
  return schema.createDocument([schema.createNode("paragraph", { id: "paragraph-1", content: [schema.createText("Hello", { id: "text-1" })] })], "document-1");
}

function firstText(document: DocumentNode): string | undefined {
  return document.content[0]?.content[0]?.text;
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

class FakeDocumentCollaborationConnection extends DisposableOwner implements DocumentCollaborationConnection {
  private readonly updateEmitter = this.own(new Emitter<DocumentCollaborationRemoteEnvelope>());
  private readonly snapshotEmitter = this.own(new Emitter<DocumentCollaborationSnapshot>());
  private readonly failureEmitter = this.own(new Emitter<Error>());

  readonly roomId = "gama-test-room";
  readonly clientId = "client-a";
  readonly initialSnapshot: DocumentCollaborationSnapshot;
  readonly onDidReceiveUpdate: Event<DocumentCollaborationRemoteEnvelope> = this.updateEmitter.event;
  readonly onDidReceiveSnapshot: Event<DocumentCollaborationSnapshot> = this.snapshotEmitter.event;
  readonly onDidFail: Event<Error> = this.failureEmitter.event;
  readonly submissions: Submission[] = [];

  constructor(readonly schema: DocumentSchema, document: DocumentNode) {
    super();
    this.initialSnapshot = Object.freeze({ roomId: this.roomId, version: 0, document });
  }

  submit(envelope: DocumentCollaborationEnvelope, document: DocumentNode, _signal: AbortSignal): Promise<DocumentCollaborationSubmitOutcome> {
    const submission = new Submission(envelope, document);
    this.submissions.push(submission);
    return submission.promise;
  }

  accept(index: number, version: number): void {
    const submission = this.submissions[index];
    assert.ok(submission, `Expected collaboration submission ${index}`);
    submission.resolve({
      kind: "accepted",
      update: {
        clientId: this.clientId,
        sequence: submission.envelope.sequence,
        baseVersion: submission.envelope.baseVersion,
        version,
        transaction: submission.envelope.transaction,
      },
    });
  }
}

class Submission {
  private readonly deferred: Deferred<DocumentCollaborationSubmitOutcome>;

  readonly promise: Promise<DocumentCollaborationSubmitOutcome>;

  constructor(readonly envelope: DocumentCollaborationEnvelope, readonly document: DocumentNode) {
    this.deferred = new Deferred<DocumentCollaborationSubmitOutcome>();
    this.promise = this.deferred.promise;
  }

  resolve(value: DocumentCollaborationSubmitOutcome): void {
    this.deferred.resolve(value);
  }
}

class Deferred<T> {
  private _resolve: ((value: T) => void) | undefined;

  readonly promise = new Promise<T>(resolve => {
    this._resolve = resolve;
  });

  resolve(value: T): void {
    this._resolve?.(value);
  }
}
