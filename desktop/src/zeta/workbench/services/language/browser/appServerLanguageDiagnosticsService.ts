import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner, toDisposable, type IDisposable } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { TextPosition, TextRange } from "../../../../editor/common/core/text.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnostic } from "../../../../editor/common/languages/languageResults.js";
import { type LanguageDiagnosticsPublisher } from "../../../../editor/common/services/languageDiagnosticsService.js";
import { type TextModel } from "../../../../editor/common/model/textModel.js";
import { type IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import { workspaceRelativePath, workspaceResourceFromPath } from "../../../../platform/files/browser/fileService.js";
import { type ILanguageApi } from "../../../../platform/language/common/languageApi.js";
import { type IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { type LanguageCodeActionDiagnosticDto, type LanguageDiagnosticsNotification } from "../../../../../../generated/app-server/types.js";
import { isAppServerLanguageId } from "./appServerLanguageSupport.js";
import { type ILanguageDiagnosticsService, type LanguageDiagnosticSnapshot } from "../common/languageDiagnosticsService.js";

const MAX_LANGUAGE_DOCUMENT_BYTES = 10 * 1024 * 1024;
const SYNCHRONIZE_DELAY_MS = 150;

interface LanguageDocumentEntry {
  readonly resource: URI;
  readonly path: string;
  readonly languageId: string;
  readonly model: TextModel;
  modelListener: IDisposable;
  references: number;
  timer: ReturnType<typeof setTimeout> | undefined;
  queue: Promise<void>;
}

interface PublishedDiagnostics {
  readonly resource: URI;
  snapshot: LanguageDiagnosticSnapshot;
}

/** Synchronizes open Code models and aggregates current diagnostics by revision. */
export class AppServerLanguageDiagnosticsService extends DisposableOwner implements ILanguageDiagnosticsService {
  private readonly entries = new Map<string, LanguageDocumentEntry>();
  private readonly serverSnapshots = new Map<string, LanguageDiagnosticSnapshot>();
  private readonly publishedDiagnostics = new Map<number, PublishedDiagnostics>();
  private readonly changeEmitter = this.own(new Emitter<URI>());
  private nextPublisherId = 1;
  readonly onDidChangeDiagnostics = this.changeEmitter.event;

  constructor(private readonly api: ILanguageApi, events: IServerEventApi, private readonly workspace: IWorkspaceContextService) {
    super();
    const subscription = events.subscribe(event => {
      if (event.method === "language/diagnostics") this.acceptDiagnostics(event.params);
    });
    this.defer(() => subscription.dispose());
    this.defer(() => {
      for (const entry of this.entries.values()) {
        if (entry.timer !== undefined) clearTimeout(entry.timer);
        entry.modelListener.dispose();
      }
      this.entries.clear();
      this.serverSnapshots.clear();
      this.publishedDiagnostics.clear();
    });
  }

  acquire(resource: URI, languageId: string, model: TextModel): IDisposable {
    if (!isAppServerLanguageId(languageId)) return toDisposable(() => undefined);
    const path = this.relativePath(resource);
    if (path === undefined || model.largeFile.tooLargeForSynchronization) return toDisposable(() => undefined);
    const key = resource.toString();
    const existing = this.entries.get(key);
    if (existing) {
      if (existing.model !== model || existing.languageId !== languageId) throw new Error("One language document resource cannot bind multiple models or languages");
      const reopening = existing.references === 0;
      if (reopening) existing.modelListener = model.onDidChange(() => this.schedule(existing, false));
      existing.references += 1;
      if (reopening) this.schedule(existing, true);
      return toDisposable(() => this.release(key, existing));
    }
    const entry: LanguageDocumentEntry = {
      resource,
      path,
      languageId,
      model,
      modelListener: toDisposable(() => undefined),
      references: 1,
      timer: undefined,
      queue: Promise.resolve(),
    };
    entry.modelListener = model.onDidChange(() => this.schedule(entry, false));
    this.entries.set(key, entry);
    this.schedule(entry, true);
    return toDisposable(() => this.release(key, entry));
  }

  getDiagnostics(resource: URI): LanguageDiagnosticSnapshot | undefined {
    return this.mergeDiagnostics(resource);
  }

  getAllDiagnostics(): readonly LanguageDiagnosticSnapshot[] {
    const resources = new Map<string, URI>();
    for (const snapshot of this.serverSnapshots.values()) resources.set(snapshot.resource.toString(), snapshot.resource);
    for (const published of this.publishedDiagnostics.values()) resources.set(published.resource.toString(), published.resource);
    return Object.freeze([...resources.values()].flatMap(resource => {
      const snapshot = this.mergeDiagnostics(resource);
      return snapshot ? [snapshot] : [];
    }));
  }

  createPublisher(resource: URI): LanguageDiagnosticsPublisher {
    const id = this.nextPublisherId++;
    let disposed = false;
    return toDisposablePublisher(
      (revision, diagnostics) => {
        if (disposed) throw new ReferenceError("Language diagnostics publisher is already disposed");
        if (!Number.isSafeInteger(revision) || revision < 1) throw new RangeError("Language diagnostics revision must be a positive safe integer");
        const snapshot = Object.freeze({ resource, revision, diagnostics: Object.freeze([...diagnostics]) });
        const current = this.publishedDiagnostics.get(id)?.snapshot;
        if (current?.revision === revision && equalDiagnostics(current.diagnostics, snapshot.diagnostics)) return;
        this.publishedDiagnostics.set(id, { resource, snapshot });
        this.changeEmitter.fire(resource);
      },
      () => {
        if (disposed) return;
        disposed = true;
        if (this.publishedDiagnostics.delete(id)) this.changeEmitter.fire(resource);
      },
    );
  }

  private schedule(entry: LanguageDocumentEntry, immediate: boolean): void {
    if (entry.references === 0 || this.entries.get(entry.resource.toString()) !== entry) return;
    if (entry.timer !== undefined) clearTimeout(entry.timer);
    if (immediate) {
      entry.timer = undefined;
      this.enqueueSynchronization(entry);
      return;
    }
    entry.timer = setTimeout(() => {
      entry.timer = undefined;
      this.enqueueSynchronization(entry);
    }, SYNCHRONIZE_DELAY_MS);
  }

  private enqueueSynchronization(entry: LanguageDocumentEntry): void {
    const snapshot = entry.model.createSnapshot();
    const text = snapshot.getText();
    if (new TextEncoder().encode(text).byteLength > MAX_LANGUAGE_DOCUMENT_BYTES) return;
    entry.queue = entry.queue.catch(() => undefined).then(async () => {
      if (entry.references === 0 || this.entries.get(entry.resource.toString()) !== entry) return;
      await this.api.synchronize({ document: { path: entry.path, languageId: entry.languageId, revision: snapshot.version, text } });
    }).catch(reportLanguageSynchronizationError);
  }

  private release(key: string, entry: LanguageDocumentEntry): void {
    if (this.entries.get(key) !== entry || entry.references === 0) return;
    entry.references -= 1;
    if (entry.references > 0) return;
    if (entry.timer !== undefined) {
      clearTimeout(entry.timer);
      entry.timer = undefined;
    }
    entry.modelListener.dispose();
    if (this.serverSnapshots.delete(key)) this.changeEmitter.fire(entry.resource);
    entry.queue = entry.queue.catch(() => undefined).then(async () => {
      if (entry.references > 0) return;
      await this.api.close({ path: entry.path });
      if (entry.references === 0 && this.entries.get(key) === entry) this.entries.delete(key);
    }).catch(reportLanguageSynchronizationError);
  }

  private acceptDiagnostics(notification: LanguageDiagnosticsNotification): void {
    const resource = workspaceResourceFromPath(this.workspaceRoot(), notification.path);
    if (!resource) return;
    const key = resource.toString();
    const entry = this.entries.get(key);
    if (!entry || notification.revision > entry.model.version) return;
    const current = this.serverSnapshots.get(key);
    if (current && current.revision > notification.revision) return;
    const diagnostics = notification.diagnostics.flatMap(diagnostic => projectDiagnostic(diagnostic, entry.model));
    this.serverSnapshots.set(key, Object.freeze({ resource, revision: notification.revision, diagnostics: Object.freeze(diagnostics) }));
    this.changeEmitter.fire(resource);
  }

  private mergeDiagnostics(resource: URI): LanguageDiagnosticSnapshot | undefined {
    const key = resource.toString();
    const candidates = [...this.publishedDiagnostics.values()].filter(candidate => candidate.resource.toString() === key).map(candidate => candidate.snapshot);
    const server = this.serverSnapshots.get(key);
    if (server) candidates.push(server);
    if (candidates.length === 0) return undefined;
    const revision = Math.max(...candidates.map(candidate => candidate.revision));
    const diagnostics = deduplicateDiagnostics(candidates.filter(candidate => candidate.revision === revision).flatMap(candidate => candidate.diagnostics));
    return Object.freeze({ resource, revision, diagnostics: Object.freeze(diagnostics) });
  }

  private relativePath(resource: URI): string | undefined {
    try {
      return workspaceRelativePath(this.workspaceRoot(), resource);
    } catch {
      return undefined;
    }
  }

  private workspaceRoot(): URI {
    const folders = this.workspace.getWorkspace().folders;
    if (folders.length !== 1) throw new Error("Language diagnostics require one workspace folder");
    return folders[0]!.uri;
  }
}

function toDisposablePublisher(update: LanguageDiagnosticsPublisher["update"], dispose: () => void): LanguageDiagnosticsPublisher {
  return { update, dispose, [Symbol.dispose]: dispose };
}

function equalDiagnostics(left: readonly LanguageDiagnostic[], right: readonly LanguageDiagnostic[]): boolean {
  return left.length === right.length && left.every((diagnostic, index) => diagnosticKey(diagnostic) === diagnosticKey(right[index]!));
}

function deduplicateDiagnostics(diagnostics: readonly LanguageDiagnostic[]): readonly LanguageDiagnostic[] {
  const seen = new Set<string>();
  return diagnostics.filter(diagnostic => {
    const key = diagnosticKey(diagnostic);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function diagnosticKey(diagnostic: LanguageDiagnostic): string {
  const range = diagnostic.range;
  return `${range.start.lineIndex}:${range.start.columnIndex}:${range.end.lineIndex}:${range.end.columnIndex}:${diagnostic.severity}:${diagnostic.message}:${diagnostic.source ?? ""}:${diagnostic.code ?? ""}`;
}

function projectDiagnostic(diagnostic: LanguageCodeActionDiagnosticDto, model: TextModel): readonly LanguageDiagnostic[] {
  const message = diagnostic.message.trim();
  if (!message) return [];
  const source = diagnostic.source?.trim() || undefined;
  const code = typeof diagnostic.code === "string" ? diagnostic.code.trim() || undefined : typeof diagnostic.code === "number" && Number.isFinite(diagnostic.code) ? diagnostic.code : undefined;
  try {
    const range = TextRange.from(TextPosition.at(diagnostic.range.start.lineIndex, diagnostic.range.start.columnIndex), TextPosition.at(diagnostic.range.end.lineIndex, diagnostic.range.end.columnIndex));
    model.offsetAt(range.start);
    model.offsetAt(range.end);
    return [Object.freeze({
      range,
      severity: diagnostic.severity as LanguageDiagnosticSeverity,
      message,
      ...(code === undefined ? {} : { code }),
      ...(source === undefined ? {} : { source }),
    })];
  } catch {
    return [];
  }
}

function reportLanguageSynchronizationError(error: unknown): void {
  console.error("App Server language document synchronization failed", error);
}
