import { throwIfCancelled } from "../../../base/common/cancellation.js";
import { type IDisposable } from "../../../base/common/lifecycle.js";
import { type URI } from "../../../base/common/uri.js";
import { type EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { type ITextFileService } from "../../../workbench/services/textfile/common/textFileService.js";
import { TextModel } from "../common/textModel.js";

interface AlphaTextModelEntry {
  readonly resource: URI;
  readonly model: TextModel;
  references: number;
}

export interface AlphaTextModelReference extends IDisposable {
  readonly resource: URI;
  readonly model: TextModel;
}

/** Shares Alpha text models by exact resource identity while references are open. */
export class AlphaTextModelService implements IDisposable {
  private readonly entries = new Map<string, AlphaTextModelEntry>();
  private disposed = false;

  async acquire(input: EditorInput, textFiles: ITextFileService, signal: AbortSignal): Promise<AlphaTextModelReference> {
    this.ensureAlive();
    validateInput(input);
    if (!textFiles || typeof textFiles.resolve !== "function") {
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
    const entry: AlphaTextModelEntry = {
      resource: input.resource,
      model: new TextModel(content.text),
      references: 0,
    };
    this.entries.set(key, entry);
    return this.reference(key, entry);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    for (const entry of this.entries.values()) entry.model.dispose();
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
      entry.model.dispose();
    };
    return Object.freeze({
      resource: entry.resource,
      model: entry.model,
      dispose,
      [Symbol.dispose]: dispose,
    });
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("AlphaTextModelService is already disposed");
  }
}

/** Product-realm model resolver shared by every Alpha editor pane. */
export const AlphaTextModels = new AlphaTextModelService();

function validateInput(input: EditorInput): void {
  if (!input || typeof input !== "object" || !input.resource || typeof input.resource.toString !== "function") {
    throw new TypeError("Alpha text model acquisition requires an editor input resource");
  }
  if (input.initialText !== undefined && typeof input.initialText !== "string") {
    throw new TypeError("Alpha editor bootstrap content must be text");
  }
}
