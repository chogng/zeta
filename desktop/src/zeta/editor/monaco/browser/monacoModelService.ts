import * as monaco from "monaco-editor";
import type { EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import { type ITextFileService } from "../../../workbench/services/textfile/common/textFileService.js";
import { monacoLanguageForInput } from "../common/monacoEditorInput.js";

interface ModelEntry {
  readonly model: monaco.editor.ITextModel;
  references: number;
}

/** A model reference that releases its Monaco model ownership exactly once. */
export interface IMonacoModelReference {
  readonly model: monaco.editor.ITextModel;
  dispose(): void;
}

const models = new Map<string, ModelEntry>();

/**
 * Acquires the shared model for an editor input.
 *
 * `initialText` is used only when the resource has no model yet. Existing
 * models are authoritative so a later input snapshot cannot erase edits made
 * by another pane.
 */
export async function acquireMonacoModel(input: EditorInput, textFiles: ITextFileService, signal: AbortSignal): Promise<IMonacoModelReference> {
  const key = input.resource.toString();
  let entry = models.get(key);
  if (!entry || entry.model.isDisposed()) {
    const content = await textFiles.resolve({
      resource: input.resource,
      ...(input.initialText === undefined ? {} : { bootstrapText: input.initialText }),
    }, signal);
    signal.throwIfAborted();
    entry = models.get(key);
    if (entry && !entry.model.isDisposed()) {
      monaco.editor.setModelLanguage(entry.model, monacoLanguageForInput(input));
      return acquireEntry(key, entry);
    }
    const model = monaco.editor.createModel(
      content.text,
      monacoLanguageForInput(input),
      monaco.Uri.parse(key),
    );
    entry = { model, references: 0 };
    models.set(key, entry);
  } else {
    monaco.editor.setModelLanguage(
      entry.model,
      monacoLanguageForInput(input),
    );
  }
  return acquireEntry(key, entry);
}

function acquireEntry(key: string, acquiredEntry: ModelEntry): IMonacoModelReference {
  acquiredEntry.references += 1;
  let released = false;
  return {
    model: acquiredEntry.model,
    dispose: () => {
      if (released) return;
      released = true;
      releaseModel(key, acquiredEntry);
    },
  };
}

function releaseModel(key: string, entry: ModelEntry): void {
  if (models.get(key) !== entry) return;
  entry.references -= 1;
  if (entry.references > 0) return;
  models.delete(key);
  entry.model.dispose();
}
