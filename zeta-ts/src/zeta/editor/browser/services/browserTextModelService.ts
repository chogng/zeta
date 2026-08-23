import { throwIfCancelled } from "../../../base/common/cancellation.js";
import { Emitter } from "../../../base/common/event.js";
import { type IDisposable } from "../../../base/common/lifecycle.js";
import { type URI } from "../../../base/common/uri.js";
import { runWhenWindowIdle } from "../../../base/browser/scheduler.js";
import { TextModelConflictError, type TextModelInput, type TextModelReference, type ITextModelService } from "../../common/services/textModelService.js";
import { TextResourceConflictError, type TextResourceChangeEvent, type ITextResourceStore } from "../../common/services/textResourceStore.js";
import { normalizeTextLineEndings } from "../../common/core/text.js";
import { TextModel, type TextModelMaintenanceOptions } from "../../common/model/textModel.js";

interface TextModelEntry {
  readonly resource: URI;
  readonly model: TextModel;
  readonly dirtyEmitter: Emitter<void>;
  readonly externalChangeEmitter: Emitter<void>;
  readonly modelChangeListener: IDisposable;
  readonly fileChangeListener: IDisposable;
  savedText: string;
  revision: string | undefined;
  lineEnding: ExternalLineEnding;
  dirty: boolean;
  hasExternalChange: boolean;
  disposed: boolean;
  saveQueue: Promise<void>;
  references: number;
}

enum ExternalLineEnding {
  LF = "\n",
  CRLF = "\r\n",
}

export interface BrowserTextModelServiceOptions {
  /** Browser-owned maintenance policy applied to newly acquired text models. */
  readonly maintenance?: TextModelMaintenanceOptions;
}

/** Shares text models by exact resource identity while references are open. */
export class BrowserTextModelService implements ITextModelService {
  private readonly entries = new Map<string, TextModelEntry>();
  private disposed = false;

  constructor(private readonly resourceStore: ITextResourceStore, private readonly options: BrowserTextModelServiceOptions = {}) {
    if (options.maintenance && typeof options.maintenance.schedule !== "function") {
      throw new TypeError("Text model maintenance requires a scheduler");
    }
    if (!resourceStore || typeof resourceStore.resolve !== "function" || typeof resourceStore.save !== "function" || typeof resourceStore.onDidChange !== "function") {
      throw new TypeError("Text model service requires a text resource store");
    }
  }

  async acquire(input: TextModelInput, signal: AbortSignal): Promise<TextModelReference> {
    this.ensureAlive();
    validateInput(input);
    throwIfCancelled(signal, "Text model acquisition was cancelled");
    const key = input.resource.toString();
    const current = this.entries.get(key);
    if (current) return this.reference(key, current);

    const content = await this.resourceStore.resolve({
      resource: input.resource,
      ...(input.initialText === undefined ? {} : { bootstrapText: input.initialText }),
    }, signal);
    throwIfCancelled(signal, "Text model acquisition was cancelled");
    this.ensureAlive();
    const concurrent = this.entries.get(key);
    if (concurrent) return this.reference(key, concurrent);
    const model = new TextModel(content.text, {
      maintenance: this.options.maintenance,
    });
    const dirtyEmitter = new Emitter<void>();
    const externalChangeEmitter = new Emitter<void>();
    const entry: TextModelEntry = {
      resource: input.resource,
      model,
      dirtyEmitter,
      externalChangeEmitter,
      modelChangeListener: model.onDidChange(() => this.refreshDirty(entry)),
      fileChangeListener: this.resourceStore.onDidChange(event => this.acceptFileChange(entry, event)),
      savedText: model.getText(),
      revision: content.revision,
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

  private reference(key: string, entry: TextModelEntry): TextModelReference {
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
      save: (signal: AbortSignal) => this.save(entry, signal),
      revert: (signal: AbortSignal) => this.revert(entry, signal),
      dispose,
      [Symbol.dispose]: dispose,
    });
  }

  private save(entry: TextModelEntry, signal: AbortSignal): Promise<void> {
    this.ensureEntryAlive(entry);
    throwIfCancelled(signal, "Text model save was cancelled");
    const savedText = entry.model.getText();
    const save = entry.saveQueue.then(async () => {
      let saved;
      try {
        saved = await this.resourceStore.save({ resource: entry.resource, text: toExternalLineEndings(savedText, entry.lineEnding), ...(entry.revision === undefined ? {} : { expectedRevision: entry.revision }) }, signal);
      } catch (error) {
        if (error instanceof TextResourceConflictError) {
          this.setExternalChange(entry, true);
          throw new TextModelConflictError(entry.resource);
        }
        throw error;
      }
      if (entry.disposed) return;
      entry.savedText = savedText;
      entry.revision = saved.revision;
      this.setExternalChange(entry, false);
      this.refreshDirty(entry);
    });
    entry.saveQueue = save.catch(() => undefined);
    return save;
  }

  private async revert(entry: TextModelEntry, signal: AbortSignal): Promise<void> {
    this.ensureEntryAlive(entry);
    throwIfCancelled(signal, "Text model revert was cancelled");
    await entry.saveQueue;
    this.ensureEntryAlive(entry);
    const content = await this.resourceStore.resolve({ resource: entry.resource }, signal);
    throwIfCancelled(signal, "Text model revert was cancelled");
    this.ensureEntryAlive(entry);
    this.applyFileContent(entry, content.text, content.revision);
    this.setExternalChange(entry, false);
  }

  private refreshDirty(entry: TextModelEntry): void {
    const dirty = entry.model.getText() !== entry.savedText;
    if (entry.dirty === dirty) return;
    entry.dirty = dirty;
    entry.dirtyEmitter.fire();
  }

  private acceptFileChange(entry: TextModelEntry, event: TextResourceChangeEvent): void {
    if (entry.disposed || (event.resources && !event.resources.some(resource => resource.toString() === entry.resource.toString()))) return;
    if (entry.dirty) {
      this.setExternalChange(entry, true);
      return;
    }
    const observedVersion = entry.model.version;
    void entry.saveQueue.then(async () => {
      const content = await this.resourceStore.resolve({ resource: entry.resource }, new AbortController().signal);
      if (entry.disposed) return;
      if (entry.dirty || entry.model.version !== observedVersion) {
        this.setExternalChange(entry, true);
        return;
      }
      if (normalizeTextLineEndings(content.text) === entry.savedText) {
        entry.lineEnding = detectExternalLineEnding(content.text);
        entry.revision = content.revision;
        this.setExternalChange(entry, false);
        return;
      }
      this.applyFileContent(entry, content.text, content.revision);
      this.setExternalChange(entry, false);
    }).catch(() => {
      if (!entry.disposed) this.setExternalChange(entry, true);
    });
  }

  private applyFileContent(entry: TextModelEntry, text: string, revision: string | undefined): void {
    entry.savedText = normalizeTextLineEndings(text);
    entry.revision = revision;
    entry.lineEnding = detectExternalLineEnding(text);
    entry.model.reset(text);
    this.refreshDirty(entry);
  }

  private setExternalChange(entry: TextModelEntry, value: boolean): void {
    if (entry.hasExternalChange === value) return;
    entry.hasExternalChange = value;
    entry.externalChangeEmitter.fire();
  }

  private disposeEntry(entry: TextModelEntry): void {
    entry.disposed = true;
    entry.modelChangeListener.dispose();
    entry.fileChangeListener.dispose();
    entry.dirtyEmitter.dispose();
    entry.externalChangeEmitter.dispose();
    entry.model.dispose();
  }

  private ensureEntryAlive(entry: TextModelEntry): void {
    if (entry.disposed) throw new ReferenceError("Text model reference is already disposed");
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("BrowserTextModelService is already disposed");
  }
}

/** Returns a browser model service with the renderer's idle maintenance policy. */
export function createBrowserTextModelService(resourceStore: ITextResourceStore): BrowserTextModelService {
  return new BrowserTextModelService(resourceStore, {
    maintenance: {
      schedule: callback => runWhenWindowIdle(
        window,
        () => callback(),
        { timeoutMs: 250 },
      ),
    },
  });
}

const modelServices = new WeakMap<ITextResourceStore, BrowserTextModelService>();

/** Shares model ownership for every pane backed by one resource store. */
export function getBrowserTextModelService(resourceStore: ITextResourceStore): BrowserTextModelService {
  const existing = modelServices.get(resourceStore);
  if (existing) return existing;
  const service = createBrowserTextModelService(resourceStore);
  modelServices.set(resourceStore, service);
  return service;
}

function validateInput(input: TextModelInput): void {
  if (!input || typeof input !== "object" || !input.resource || typeof input.resource.toString !== "function") {
    throw new TypeError("Text model acquisition requires an editor input resource");
  }
  if (input.initialText !== undefined && typeof input.initialText !== "string") {
    throw new TypeError("Editor bootstrap content must be text");
  }
}

function detectExternalLineEnding(text: string): ExternalLineEnding {
  return text.includes("\r\n") ? ExternalLineEnding.CRLF : ExternalLineEnding.LF;
}

function toExternalLineEndings(text: string, lineEnding: ExternalLineEnding): string {
  return lineEnding === ExternalLineEnding.CRLF ? text.replaceAll("\n", "\r\n") : text;
}
