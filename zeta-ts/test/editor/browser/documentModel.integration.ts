import { URI } from "../../../src/zeta/base/common/uri.js";
import { Emitter, type Event } from "../../../src/zeta/base/common/event.js";
import { DisposableOwner } from "../../../src/zeta/base/common/lifecycle.js";
import { EmbeddedTextEditorFactory } from "../../../src/zeta/workbench/contrib/codeEditor/browser/embeddedTextEditor.js";
import { createDefaultDocumentSchema } from "../../../src/zeta/editor/editor.api.js";
import { createTextNode } from "../../../src/zeta/editor/editor.api.js";
import { DocumentModel } from "../../../src/zeta/editor/editor.api.js";
import "../../../src/zeta/editor/editor.academic.all.js";
import { DocumentEditorPane } from "../../../src/zeta/workbench/contrib/documentEditor/browser/documentEditorPane.js";
import type { DocumentNode } from "../../../src/zeta/editor/common/model/document.js";
import { serializeDocument } from "../../../src/zeta/editor/common/model/documentSerialization.js";
import type { DocumentSchema } from "../../../src/zeta/editor/common/model/documentSchema.js";
import type { DocumentCollaborationConnection } from "../../../src/zeta/editor/common/services/documentCollaborationService.js";
import type { DocumentCollaborationInvite } from "../../../src/zeta/editor/common/services/documentCollaborationService.js";
import type { DocumentCollaborationMember } from "../../../src/zeta/editor/common/services/documentCollaborationService.js";
import type { DocumentCollaborationOpenInput } from "../../../src/zeta/editor/common/services/documentCollaborationService.js";
import type { DocumentCollaborationPresence } from "../../../src/zeta/editor/common/services/documentCollaborationService.js";
import type { DocumentCollaborationRoomRole } from "../../../src/zeta/editor/common/services/documentCollaborationService.js";
import type { DocumentSelection } from "../../../src/zeta/editor/common/core/documentSelection.js";
import type { DocumentCollaborationSnapshot } from "../../../src/zeta/editor/common/services/documentCollaborationService.js";
import type { DocumentCollaborationRemoteEnvelope } from "../../../src/zeta/editor/contrib/collaboration/common/protocol.js";
import type { DocumentCollaborationEnvelope } from "../../../src/zeta/editor/contrib/collaboration/common/protocol.js";
import type { DocumentCollaborationSubmitOutcome } from "../../../src/zeta/editor/common/services/documentCollaborationService.js";
import type { IDocumentCollaborationService } from "../../../src/zeta/editor/common/services/documentCollaborationService.js";
import { MemoryTextFiles } from "./memoryTextFiles.js";

interface IntegrationHarness {
  readonly apiDocumentType: string;
  getTextBlockText(): string | undefined;
  getStructuredBlockTexts(): readonly string[];
  getStructuredFirstTextMarks(): readonly { readonly type: string; readonly attrs: Readonly<Record<string, string | number | boolean | null>> }[];
  getStructuredSelection(): unknown;
  saveTextBlock(): Promise<void>;
  getSavedTextBlock(): string;
  dispose(): void;
}

declare global {
  interface Window {
    zetaDocumentModelIntegration: IntegrationHarness;
  }
}

class BrowserDocumentCollaborationService extends DisposableOwner implements IDocumentCollaborationService {
  async open(input: DocumentCollaborationOpenInput, _signal: AbortSignal): Promise<DocumentCollaborationConnection> {
    return new BrowserDocumentCollaborationConnection(input.schema, input.clientId, input.document, input.roomId ?? "editor-browser-room", input.target?.kind === "remote");
  }
}

class BrowserDocumentCollaborationConnection extends DisposableOwner implements DocumentCollaborationConnection {
  private readonly updates = this.own(new Emitter<DocumentCollaborationRemoteEnvelope>());
  private readonly snapshots = this.own(new Emitter<DocumentCollaborationSnapshot>());
  private readonly presences = this.own(new Emitter<readonly DocumentCollaborationPresence[]>());
  private readonly failures = this.own(new Emitter<Error>());
  private version = 0;

  readonly initialSnapshot;
  readonly canEdit = true;
  readonly principalId: string | undefined;
  readonly onDidReceiveUpdate: Event<DocumentCollaborationRemoteEnvelope> = this.updates.event;
  readonly onDidReceiveSnapshot: Event<DocumentCollaborationSnapshot> = this.snapshots.event;
  readonly onDidReceivePresence: Event<readonly DocumentCollaborationPresence[]> = this.presences.event;
  readonly onDidFail: Event<Error> = this.failures.event;
  readonly currentPresence: readonly DocumentCollaborationPresence[] = [];

  constructor(readonly schema: DocumentSchema, readonly clientId: string, document: DocumentNode, readonly roomId: string, readonly canManageMembers: boolean) {
    super();
    this.principalId = canManageMembers ? "browser-owner" : undefined;
    this.initialSnapshot = Object.freeze({ roomId, version: this.version, document });
  }

  async submit(envelope: DocumentCollaborationEnvelope, _document: DocumentNode, _signal: AbortSignal): Promise<DocumentCollaborationSubmitOutcome> {
    this.version += 1;
    return {
      kind: "accepted",
      update: {
        clientId: this.clientId,
        sequence: envelope.sequence,
        baseVersion: envelope.baseVersion,
        version: this.version,
        transaction: envelope.transaction,
      },
    };
  }

  async updatePresence(_selection: DocumentSelection | undefined, _signal: AbortSignal): Promise<void> {}

  async createInvite(displayName: string, role: DocumentCollaborationRoomRole, _signal: AbortSignal): Promise<DocumentCollaborationInvite> {
    if (!this.canManageMembers) throw new Error("This collaboration member cannot create room invitations");
    return Object.freeze({ roomId: this.roomId, principalId: "browser-member", displayName, role, accessToken: "editor-browser-member-token" });
  }

  async listMembers(_signal: AbortSignal): Promise<readonly DocumentCollaborationMember[]> {
    if (!this.canManageMembers) throw new Error("This collaboration member cannot inspect room members");
    return Object.freeze([
      Object.freeze({ principalId: "browser-owner", displayName: "Browser owner", role: "owner" }),
      Object.freeze({ principalId: "browser-member", displayName: "Writer", role: "editor" }),
    ]);
  }

  async rotateMemberAccessToken(principalId: string, _signal: AbortSignal): Promise<DocumentCollaborationInvite> {
    if (!this.canManageMembers) throw new Error("This collaboration member cannot manage room credentials");
    return Object.freeze({ roomId: this.roomId, principalId, displayName: principalId === "browser-owner" ? "Browser owner" : "Writer", role: principalId === "browser-owner" ? "owner" : "editor", accessToken: "editor-browser-rotated-token" });
  }

  async revokeMember(_principalId: string, _signal: AbortSignal): Promise<void> {
    if (!this.canManageMembers) throw new Error("This collaboration member cannot manage room credentials");
  }
}

const schema = createDefaultDocumentSchema();
const apiDocument = schema.createDocument([schema.createNode("paragraph", { content: [schema.createText("editor-api")] })]);
const apiModel = new DocumentModel(schema, apiDocument);
const textBlockResource = URI.parse("inmemory://editor/text-block.zeta-academic");
const structuredResource = URI.parse("inmemory://editor/document.zeta-academic");
const textBlockDocument = schema.createDocument([schema.createNode("textBlock", {
  attrs: { language: "typescript" },
  content: [createTextNode("editor-text", "const editor = 1;")],
  id: "editor-text-block",
})], "editor-text-document");
const textBlockFiles = new MemoryTextFiles(textBlockResource, serializeDocument(textBlockDocument, schema));
const structuredFiles = new MemoryTextFiles(structuredResource, "Title\nBody");
const textBlockPane = new DocumentEditorPane(textBlockFiles, { embeddedTextEditorFactory: new EmbeddedTextEditorFactory() });
const structuredPane = new DocumentEditorPane(structuredFiles, { documentCollaborationService: new BrowserDocumentCollaborationService() });

textBlockPane.create(requiredElement("#text-block"));
structuredPane.create(requiredElement("#document-editor"));
textBlockPane.layout({ width: 900, height: 300 });
structuredPane.layout({ width: 900, height: 300 });
await textBlockPane.setInput({ resource: textBlockResource, label: "snippet.ts" }, new AbortController().signal);
await structuredPane.setInput({ resource: structuredResource, label: "paper" }, new AbortController().signal);

window.zetaDocumentModelIntegration = {
  apiDocumentType: apiModel.document.type,
  getTextBlockText: () => textBlockPane.getDocument().content[0]?.content[0]?.text,
  getStructuredBlockTexts: () => structuredPane.getDocument().content.map(block => block.content.find(child => child.text !== undefined)?.text ?? ""),
  getStructuredFirstTextMarks: () => structuredPane.getDocument().content[0]?.content[0]?.marks ?? [],
  getStructuredSelection: () => structuredPane.getDocumentSelection(),
  saveTextBlock: () => textBlockPane.save(),
  getSavedTextBlock: () => textBlockFiles.read(textBlockResource),
  dispose: () => {
    apiModel.dispose();
    textBlockPane.dispose();
    structuredPane.dispose();
    textBlockFiles.dispose();
    structuredFiles.dispose();
  },
};

function requiredElement(selector: string): HTMLElement {
  const element = document.querySelector<HTMLElement>(selector);
  if (!element) throw new Error(`Missing editor integration root '${selector}'`);
  return element;
}
