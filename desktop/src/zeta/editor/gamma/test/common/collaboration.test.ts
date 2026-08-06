import assert from "node:assert/strict";
import test from "node:test";
import { rebaseDocumentTransaction } from "../../contrib/collaboration/common/rebase.js";
import { createDefaultDocumentSchema, type DocumentSchema } from "../../common/schema.js";
import { nodeSelection, textSelection } from "../../common/selection.js";
import { DocumentTransaction } from "../../common/transaction.js";

function createDocument(schema: DocumentSchema) {
  return schema.createDocument([
    schema.createNode("paragraph", { id: "paragraph-1", content: [schema.createText("Hello", { id: "text-1" })] }),
    schema.createNode("paragraph", { id: "paragraph-2", content: [schema.createText("Second", { id: "text-2" })] }),
  ], "document-1");
}

test("Gamma collaboration rebases text offsets and selections through remote insertion", () => {
  const schema = createDefaultDocumentSchema();
  const base = createDocument(schema);
  const local = new DocumentTransaction()
    .replaceText("text-1", 1, 1, "L")
    .withSelection(textSelection({ nodeId: "text-1", offset: 2 }));
  const remote = new DocumentTransaction().replaceText("text-1", 0, 0, "R");

  const result = rebaseDocumentTransaction(base, schema, local, remote);

  assert.equal(result.remoteDocument.content[0]?.content[0]?.text, "RHello");
  assert.equal(result.document.content[0]?.content[0]?.text, "RHLello");
  assert.deepEqual(result.transaction.steps, [{ kind: "replaceText", nodeId: "text-1", from: 2, to: 2, text: "L" }]);
  assert.deepEqual(result.transaction.selection, textSelection({ nodeId: "text-1", offset: 3 }));
  assert.deepEqual(result.droppedSteps, []);
});

test("Gamma collaboration shifts structural insertion indices through remote siblings", () => {
  const schema = createDefaultDocumentSchema();
  const base = createDocument(schema);
  const localNode = schema.createNode("paragraph", { id: "paragraph-local", content: [schema.createText("Local", { id: "text-local" })] });
  const remoteNode = schema.createNode("paragraph", { id: "paragraph-remote", content: [schema.createText("Remote", { id: "text-remote" })] });
  const local = new DocumentTransaction().insertNode("document-1", 1, localNode);
  const remote = new DocumentTransaction().insertNode("document-1", 0, remoteNode);

  const result = rebaseDocumentTransaction(base, schema, local, remote);

  assert.deepEqual(result.document.content.map(node => node.id), ["paragraph-remote", "paragraph-1", "paragraph-local", "paragraph-2"]);
  assert.equal(result.transaction.steps[0]?.kind, "insertNode");
  assert.equal(result.transaction.steps[0]?.kind === "insertNode" ? result.transaction.steps[0].index : -1, 2);
});

test("Gamma collaboration drops local steps whose targets were deleted remotely", () => {
  const schema = createDefaultDocumentSchema();
  const base = createDocument(schema);
  const local = new DocumentTransaction().setNodeAttributes("paragraph-2", { alignment: "center" });
  const remote = new DocumentTransaction().deleteNode("paragraph-2");

  const result = rebaseDocumentTransaction(base, schema, local, remote);

  assert.deepEqual(result.document.content.map(node => node.id), ["paragraph-1"]);
  assert.deepEqual(result.transaction.steps, []);
  assert.deepEqual(result.droppedSteps, local.steps);
});

test("Gamma collaboration preserves local transaction dependencies and local node selection", () => {
  const schema = createDefaultDocumentSchema();
  const base = createDocument(schema);
  const localNode = schema.createNode("paragraph", { id: "paragraph-local", content: [schema.createText("Local", { id: "text-local" })] });
  const local = new DocumentTransaction()
    .insertNode("document-1", 1, localNode)
    .setNodeAttributes("paragraph-local", { alignment: "center" })
    .withSelection(nodeSelection("paragraph-local"));
  const remote = new DocumentTransaction().insertNode("document-1", 0, schema.createNode("paragraph", { id: "paragraph-remote", content: [schema.createText("Remote", { id: "text-remote" })] }));

  const result = rebaseDocumentTransaction(base, schema, local, remote);

  assert.equal(result.document.content[2]?.attrs.alignment, "center");
  assert.deepEqual(result.transaction.selection, nodeSelection("paragraph-local"));
  assert.equal(result.transaction.steps.length, 2);
});

test("Gamma collaboration maps non-overlapping replacements after a remote replacement", () => {
  const schema = createDefaultDocumentSchema();
  const base = schema.createDocument([schema.createNode("paragraph", { id: "paragraph-1", content: [schema.createText("Hello", { id: "text-1" })] })], "document-1");
  const local = new DocumentTransaction().replaceText("text-1", 2, 3, "X");
  const remote = new DocumentTransaction().replaceText("text-1", 0, 1, "Remote");

  const result = rebaseDocumentTransaction(base, schema, local, remote);

  assert.equal(result.document.content[0]?.content[0]?.text, "RemoteeXlo");
  assert.deepEqual(result.transaction.steps, [{ kind: "replaceText", nodeId: "text-1", from: 7, to: 8, text: "X" }]);
});
