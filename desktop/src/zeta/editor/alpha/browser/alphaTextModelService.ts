import { throwIfCancelled } from "../../../base/common/cancellation.js";
import { Emitter, type Event } from "../../../base/common/event.js";
import { type IDisposable } from "../../../base/common/lifecycle.js";
import { type URI } from "../../../base/common/uri.js";
import { runWhenWindowIdle } from "../../../base/browser/scheduler.js";
import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { type ITextFileService } from "../../../workbench/services/textfile/common/textFileService.js";
import { type IFileChangeEvent } from "../../../platform/files/common/files.js";
import { normalizeTextLineEndings, TextRange } from "../common/text.js";
import { TextModel, type TextModelMaintenanceOptions } from "../common/textModel.js";

interface AlphaTextModelEntry {
  readonly resource: URI;
  readonly model: TextModel;
  readonly dirtyEmitter: Emitter<void>;
  readonly externalChangeEmitter: Emitter<void>;
  readonly modelChangeListener: IDisposable;
  readonly fileChangeListener: IDisposable;
  savedText: string;
  lineEnding: AlphaExternalLineEnding;
  dirty: boolean;
  hasExternalChange: boolean;
  disposed: boolean;
  saveQueue: Promise<void>;
  references: number;
}

enum AlphaExternalLineEnding {
  LF = "\n",
  CRLF = "\r\n",
}

export interface AlphaTextModelReference extends IDisposable {
  readonly resource: URI;
  readonly model: TextModel;
  readonly isDirty: boolean;
  readonly onDidChangeDirty: Event<void>;
  readonly hasExternalChange: boolean;
  readonly onDidChangeExternalChange: Event<void>;
  save(textFiles: ITextFileService, signal: AbortSignal): Promise<void>;
  revert(textFiles: ITextFileService, signal: AbortSignal): Promise<void>;
}

export interface AlphaTextModelServiceOptions {
  /** Browser-owned maintenance policy applied to newly acquired text models. */
  readonly maintenance?: TextModelMaintenanceOptions;
}

/** Reports that the resource changed after Alpha established its saved baseline. */
export class AlphaTextModelConflictError extends Error {
  constructor(readonly resource: URI) {
    super(`Cannot save '${resource.toString()}' because it changed outside Alpha`);
    this.name = "AlphaTextModelConflictError";
  }
}

/** Shares Alpha text models by exact resource identity while references are open. */
export class AlphaTextModelService implements IDisposable {
  private readonly entries = new Map<string, AlphaTextModelEntry>();
  private disposed = false;

  constructor(private readonly options: AlphaTextModelServiceOptions = {}) {
    if (options.maintenance && typeof options.maintenance.schedule !== "function") {
      throw new TypeError("Alpha text model maintenance requires a scheduler");
    }
  }

  async acquire(input: EditorInput, textFiles: ITextFileService, signal: AbortSignal): Promise<AlphaTextModelReference> {
    this.ensureAlive();
    validateInput(input);
    if (!textFiles || typeof textFiles.resolve !== "function" || typeof textFiles.save !== "function" || typeof textFiles.onDidChangeFiles !== "function") {
      throw new TypeError("Alpha text model service requires a text file service");
    }
    throwIfCancelled(signal, "Alpha text model acquisition was cancelled");
    const key = input.resource.toString();
    const current = this.entries.get(key);
    if (current) return this.reference(key, current);

    const content = await textFiles.resolve({
      resource: input.resource,
      ...(input.initialText === undefined ? {} : { bootstrapText: input.initialText }),
    }, signal);
    throwIfCancelled(signal, "Alpha text model acquisition was cancelled");
    this.ensureAlive();
    const concurrent = this.entries.get(key);
    if (concurrent) return this.reference(key, concurrent);
    const model = new TextModel(content.text, {
      maintenance: this.options.maintenance,
    });
    const dirtyEmitter = new Emitter<void>();
    const externalChangeEmitter = new Emitter<void>();
    const entry: AlphaTextModelEntry = {
      resource: input.resource,
      model,
      dirtyEmitter,
      externalChangeEmitter,
      modelChangeListener: model.onDidChange(() => this.refreshDirty(entry)),
      fileChangeListener: textFiles.onDidChangeFiles(event => this.acceptFileChange(entry, textFiles, event)),
      savedText: model.getText(),
      lineEnding: detectExternalLineEnding(content.text),
      dirty: false,
      hasExternalChange: false,
      disposed: false,
      saveQueue: Promise.resolve(),
      references: 0,
    };
    this.entries.set(key, entry);
    return this.reference(key, entry);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    for (const entry of this.entries.values()) this.disposeEntry(entry);
    this.entries.clear();
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  private reference(key: string, entry: AlphaTextModelEntry): AlphaTextModelReference {
    entry.references += 1;
    let released = false;
    const dispose = (): void => {
      if (released) return;
      released = true;
      if (this.entries.get(key) !== entry) return;
      entry.references -= 1;
      if (entry.references > 0) return;
      this.entries.delete(key);
      this.disposeEntry(entry);
    };
    return Object.freeze({
      resource: entry.resource,
      model: entry.model,
      get isDirty(): boolean {
        return entry.dirty;
      },
      onDidChangeDirty: entry.dirtyEmitter.event,
      get hasExternalChange(): boolean {
        return entry.hasExternalChange;
      },
      onDidChangeExternalChange: entry.externalChangeEmitter.event,
      save: (textFiles: ITextFileService, signal: AbortSignal) => this.save(entry, textFiles, signal),
      revert: (textFiles: ITextFileService, signal: AbortSignal) => this.revert(entry, textFiles, signal),
      dispose,
      [Symbol.dispose]: dispose,
    });
  }

  private save(entry: AlphaTextModelEntry, textFiles: ITextFileService, signal: AbortSignal): Promise<void> {
    this.ensureTextFileService(textFiles);
    this.ensureEntryAlive(entry);
    throwIfCancelled(signal, "Alpha text model save was cancelled");
    const savedText = entry.model.getText();
    const save = entry.saveQueue.then(async () => {
      const current = await textFiles.resolve({ resource: entry.resource }, signal);
      if (normalizeTextLineEndings(current.text) !== entry.savedText) {
        this.setExternalChange(entry, true);
        throw new AlphaTextModelConflictError(entry.resource);
      }
      await textFiles.save({ resource: entry.resource, text: toExternalLineEndings(savedText, entry.lineEnding) }, signal);
      if (entry.disposed) return;
      entry.savedText = savedText;
      this.setExternalChange(entry, false);
      this.refreshDirty(entry);
    });
    entry.saveQueue = save.catch(() => undefined);
    return save;
  }

  private async revert(entry: AlphaTextModelEntry, textFiles: ITextFileService, signal: AbortSignal): Promise<void> {
    this.ensureTextFileService(textFiles);
    this.ensureEntryAlive(entry);
    throwIfCancelled(signal, "Alpha text model revert was cancelled");
    await entry.saveQueue;
    this.ensureEntryAlive(entry);
    const content = await textFiles.resolve({ resource: entry.resource }, signal);
    throwIfCancelled(signal, "Alpha text model revert was cancelled");
    this.ensureEntryAlive(entry);
    this.applyFileContent(entry, content.text);
    this.setExternalChange(entry, false);
  }

  private refreshDirty(entry: AlphaTextModelEntry): void {
    const dirty = entry.model.getText() !== entry.savedText;
    if (entry.dirty === dirty) return;
    entry.dirty = dirty;
    entry.dirtyEmitter.fire();
  }

  private acceptFileChange(entry: AlphaTextModelEntry, textFiles: ITextFileService, event: IFileChangeEvent): void {
    if (entry.disposed || (event.resources && !event.resources.some(resource => resource.toString() === entry.resource.toString()))) return;
    if (entry.dirty) {
      this.setExternalChange(entry, true);
      return;
    }
    const observedVersion = entry.model.version;
    void entry.saveQueue.then(async () => {
      const content = await textFiles.resolve({ resource: entry.resource }, new AbortController().signal);
      if (entry.disposed) return;
      if (entry.dirty || entry.model.version !== observedVersion) {
        this.setExternalChange(entry, true);
        return;
      }
      if (normalizeTextLineEndings(content.text) === entry.savedText) {
        entry.lineEnding = detectExternalLineEnding(content.text);
        this.setExternalChange(entry, false);
        return;
      }
      this.applyFileContent(entry, content.text);
      this.setExternalChange(entry, false);
    }).catch(() => {
      if (!entry.disposed) this.setExternalChange(entry, true);
    });
  }

  private applyFileContent(entry: AlphaTextModelEntry, text: string): void {
    entry.savedText = normalizeTextLineEndings(text);
    entry.lineEnding = detectExternalLineEnding(text);
    const snapshot = entry.model.createSnapshot();
    entry.model.applyEdits([{
      range: TextRange.from(entry.model.positionAt(0), entry.model.positionAt(snapshot.length)),
      text,
    }]);
    this.refreshDirty(entry);
  }

  private setExternalChange(entry: AlphaTextModelEntry, value: boolean): void {
    if (entry.hasExternalChange === value) return;
    entry.hasExternalChange = value;
    entry.externalChangeEmitter.fire();
  }

  private disposeEntry(entry: AlphaTextModelEntry): void {
    entry.disposed = true;
    entry.modelChangeListener.dispose();
    entry.fileChangeListener.dispose();
    entry.dirtyEmitter.dispose();
    entry.externalChangeEmitter.dispose();
    entry.model.dispose();
  }

  private ensureTextFileService(textFiles: ITextFileService): void {
    if (!textFiles || typeof textFiles.resolve !== "function" || typeof textFiles.save !== "function" || typeof textFiles.onDidChangeFiles !== "function") {
      throw new TypeError("Alpha text model service requires a text file service");
    }
  }

  private ensureEntryAlive(entry: AlphaTextModelEntry): void {
    if (entry.disposed) throw new ReferenceError("Alpha text model reference is already disposed");
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("AlphaTextModelService is already disposed");
  }
}

/** Product-realm model resolver shared by every Alpha editor pane. */
export const AlphaTextModels = new AlphaTextModelService({
  maintenance: {
    schedule: callback => runWhenWindowIdle(
      window,
      () => callback(),
      { timeoutMs: 250 },
    ),
  },
});

function validateInput(input: EditorInput): void {
  if (!input || typeof input !== "object" || !input.resource || typeof input.resource.toString !== "function") {
    throw new TypeError("Alpha text model acquisition requires an editor input resource");
  }
  if (input.initialText !== undefined && typeof input.initialText !== "string") {
    throw new TypeError("Alpha editor bootstrap content must be text");
  }
}

function detectExternalLineEnding(text: string): AlphaExternalLineEnding {
  return text.includes("\r\n") ? AlphaExternalLineEnding.CRLF : AlphaExternalLineEnding.LF;
}

function toExternalLineEndings(text: string, lineEnding: AlphaExternalLineEnding): string {
  return lineEnding === AlphaExternalLineEnding.CRLF ? text.replaceAll("\n", "\r\n") : text;
}
