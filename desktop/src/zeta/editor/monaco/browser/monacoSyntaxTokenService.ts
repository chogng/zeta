import * as monaco from "monaco-editor";
import { DisposableOwner, toDisposable } from "../../../base/common/lifecycle.js";
import type { ZetaRendererApi } from "../../../platform/app-server/common/renderer-api.js";
import type { SyntaxTextEditDto, SyntaxTokenSnapshotDto } from "../../../../../generated/app-server/types.js";

/** Order mirrors `syntax_operations::token_type` in the App Server. */
const RUST_SYNTAX_TOKEN_TYPES = [
  "decorator",
  "comment",
  "enumMember",
  "class",
  "string",
  "function",
  "keyword",
  "label",
  "namespace",
  "number",
  "operator",
  "property",
  "type",
  "variable",
] as const;

/** Projects revision-bound Rust syntax tokens from the App Server into Monaco. */
export class MonacoSyntaxTokenService extends DisposableOwner implements monaco.languages.DocumentSemanticTokensProvider {
  private readonly sessions = new Map<string, ModelSyntaxSession>();

  constructor(private readonly api: ZetaRendererApi) {
    super();
    const registration = monaco.languages.registerDocumentSemanticTokensProvider("rust", this);
    this.own(toDisposable(() => registration.dispose()));
    this.defer(() => {
      for (const session of this.sessions.values()) session.dispose();
      this.sessions.clear();
    });
  }

  getLegend(): monaco.languages.SemanticTokensLegend {
    return {
      tokenTypes: [...RUST_SYNTAX_TOKEN_TYPES],
      tokenModifiers: [],
    };
  }

  async provideDocumentSemanticTokens(model: monaco.editor.ITextModel, _lastResultId: string | null, token: monaco.CancellationToken): Promise<monaco.languages.SemanticTokens | null> {
    if (token.isCancellationRequested || model.isDisposed()) return null;
    const session = this.session(model);
    const snapshot = await session.currentSnapshot();
    if (token.isCancellationRequested || !snapshot || snapshot.revision !== model.getVersionId()) {
      return null;
    }
    return {
      resultId: snapshot.resultId,
      data: Uint32Array.from(snapshot.data),
    };
  }

  releaseDocumentSemanticTokens(_resultId: string | undefined): void {}

  private session(model: monaco.editor.ITextModel): ModelSyntaxSession {
    const key = model.uri.toString();
    const existing = this.sessions.get(key);
    if (existing?.model === model) return existing;
    existing?.dispose();
    const session = new ModelSyntaxSession(this.api, model, () => {
      if (this.sessions.get(key) === session) this.sessions.delete(key);
    });
    this.sessions.set(key, session);
    return session;
  }
}

class ModelSyntaxSession extends DisposableOwner {
  private queue: Promise<void> = Promise.resolve();
  private observedRevision: number;
  private backendRevision: number | undefined;
  private snapshot: SyntaxTokenSnapshotDto | undefined;
  private closed = false;

  constructor(private readonly api: ZetaRendererApi, readonly model: monaco.editor.ITextModel, onDidClose: () => void) {
    super();
    const initialRevision = model.getVersionId();
    const initialText = model.getValue();
    this.observedRevision = initialRevision;
    this.defer(() => {
      this.closed = true;
      onDidClose();
      void this.queue.then(() => this.api.syntax.close({ documentId: this.model.id })).catch(() => undefined);
    });
    const changeListener = model.onDidChangeContent((event) => this.queueChange(event));
    const disposeListener = model.onWillDispose(() => this.dispose());
    this.own(toDisposable(() => changeListener.dispose()));
    this.own(toDisposable(() => disposeListener.dispose()));
    this.enqueue(async () => {
      const snapshot = await this.api.syntax.open({
        documentId: this.model.id,
        documentUri: this.model.uri.toString(),
        language: "rust",
        revision: initialRevision,
        text: initialText,
      });
      this.acceptSnapshot(snapshot, initialRevision);
    });
  }

  async currentSnapshot(): Promise<SyntaxTokenSnapshotDto | undefined> {
    await this.awaitQueue();
    if (this.closed || this.model.isDisposed()) return undefined;
    if (this.backendRevision !== this.model.getVersionId()) {
      this.enqueue(() => this.reopenCurrent());
      await this.awaitQueue();
    }
    return this.backendRevision === this.model.getVersionId() ? this.snapshot : undefined;
  }

  private queueChange(event: monaco.editor.IModelContentChangedEvent): void {
    const previousRevision = this.observedRevision;
    const revision = this.model.getVersionId();
    this.observedRevision = revision;
    const edits: SyntaxTextEditDto[] = event.changes.map((change) => ({
      startUtf16: change.rangeOffset,
      endUtf16: change.rangeOffset + change.rangeLength,
      text: change.text,
    }));
    this.enqueue(async () => {
      if (this.backendRevision !== previousRevision) {
        await this.reopenCurrent();
        if (this.backendRevision !== previousRevision) return;
      }
      const snapshot = await this.api.syntax.change({
        documentId: this.model.id,
        previousRevision,
        revision,
        edits,
      });
      this.acceptSnapshot(snapshot, revision);
    });
  }

  private async reopenCurrent(): Promise<void> {
    if (this.closed || this.model.isDisposed()) return;
    const revision = this.model.getVersionId();
    const snapshot = await this.api.syntax.open({
      documentId: this.model.id,
      documentUri: this.model.uri.toString(),
      language: "rust",
      revision,
      text: this.model.getValue(),
    });
    this.acceptSnapshot(snapshot, revision);
  }

  private acceptSnapshot(snapshot: SyntaxTokenSnapshotDto, expectedRevision: number): void {
    if (snapshot.revision !== expectedRevision || snapshot.data.length % 5 !== 0 || snapshot.data.some((value) => !Number.isInteger(value) || value < 0 || value > 0xffff_ffff)) {
      throw new TypeError("App Server returned an invalid syntax-token snapshot");
    }
    this.snapshot = snapshot;
    this.backendRevision = snapshot.revision;
  }

  private enqueue(operation: () => Promise<void>): void {
    this.queue = this.queue.then(async () => {
      if (!this.closed) await operation();
    }).catch(() => {
      this.snapshot = undefined;
      this.backendRevision = undefined;
    });
  }

  private async awaitQueue(): Promise<void> {
    for (;;) {
      const pending = this.queue;
      await pending;
      if (pending === this.queue) return;
    }
  }
}
