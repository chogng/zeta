import "./media/monacoEditor.css";
import "./monacoEnvironment.js";
import * as monaco from "monaco-editor";
import type {
  IDimension,
} from "../../../base/browser/geometry.js";
import {
  DisposableOwner,
} from "../../../base/common/lifecycle.js";
import type {
  EditorInput,
} from "../../../workbench/browser/parts/editor/editorInput.js";
import type {
  IEditorPane,
} from "../../../workbench/browser/parts/editor/editorPane.js";
import {
  EditorPaneVisibility,
} from "../../../workbench/browser/parts/editor/editorPane.js";
import {
  MONACO_EDITOR_ID,
  monacoLanguageForInput,
} from "../common/monacoEditorInput.js";

/** Browser host for the customizable Monaco editor subsystem. */
export class MonacoEditorPane extends DisposableOwner
  implements IEditorPane {
  readonly id = MONACO_EDITOR_ID;

  #container: HTMLDivElement | undefined;
  #editor: monaco.editor.IStandaloneCodeEditor | undefined;
  #model: monaco.editor.ITextModel | undefined;
  #dimension: IDimension = { width: 0, height: 0 };

  create(parent: HTMLElement): void {
    if (this.#container) {
      throw new ReferenceError("MonacoEditorPane has already been created");
    }
    const container = parent.ownerDocument.createElement("div");
    container.className = "zeta-monaco-editor-pane";
    parent.append(container);
    this.#container = container;
    this.#editor = monaco.editor.create(container, {
      automaticLayout: false,
      minimap: { enabled: true },
      model: null,
      scrollBeyondLastLine: false,
      theme: "vs-dark",
    });
    this.defer(() => {
      this.#clearModel();
      this.#editor?.dispose();
      this.#editor = undefined;
      container.remove();
      this.#container = undefined;
    });
  }

  async setInput(
    input: EditorInput,
    signal: AbortSignal,
  ): Promise<void> {
    const editor = this.#requireEditor();
    throwIfAborted(signal);
    const model = monaco.editor.createModel(
      input.initialText ?? "",
      monacoLanguageForInput(input),
    );
    if (signal.aborted) {
      model.dispose();
      throw abortError();
    }
    this.#clearModel();
    this.#model = model;
    editor.setModel(model);
    editor.updateOptions({
      ariaLabel: input.label ?? resourceLabel(input),
    });
    editor.layout(this.#dimension);
  }

  clearInput(): void {
    this.#clearModel();
  }

  layout(dimension: IDimension): void {
    this.#dimension = {
      width: Math.max(0, dimension.width),
      height: Math.max(0, dimension.height),
    };
    this.#editor?.layout(this.#dimension);
  }

  setVisible(visibility: EditorPaneVisibility): void {
    if (!this.#container) return;
    this.#container.hidden = visibility === EditorPaneVisibility.Hidden;
    if (visibility === EditorPaneVisibility.Visible) {
      this.#editor?.layout(this.#dimension);
    }
  }

  focus(): void {
    this.#requireEditor().focus();
  }

  getValue(): string {
    return this.#model?.getValue() ?? "";
  }

  setValue(value: string): void {
    const model = this.#model;
    if (!model) {
      throw new ReferenceError("MonacoEditorPane has no active input");
    }
    model.setValue(value);
  }

  #clearModel(): void {
    this.#editor?.setModel(null);
    this.#model?.dispose();
    this.#model = undefined;
  }

  #requireEditor(): monaco.editor.IStandaloneCodeEditor {
    if (!this.#editor) {
      throw new ReferenceError("MonacoEditorPane has not been created");
    }
    return this.#editor;
  }
}

function resourceLabel(input: EditorInput): string {
  const path = decodeURIComponent(input.resource.path);
  return path.slice(path.lastIndexOf("/") + 1) || "Code editor";
}

function throwIfAborted(signal: AbortSignal): void {
  if (signal.aborted) throw abortError();
}

function abortError(): DOMException {
  return new DOMException("Editor input loading was aborted", "AbortError");
}
