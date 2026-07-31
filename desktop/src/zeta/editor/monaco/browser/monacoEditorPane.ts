import "./media/monacoEditor.css";
import "./monacoEnvironment.js";
import * as monaco from "monaco-editor";
import type { IDimension } from "../../../base/browser/geometry.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { assertDefined } from "../../../base/common/types.js";
import type { IConfigurationService } from "../../../platform/configuration/common/configuration.js";
import { type ITextFileService } from "../../../workbench/services/textfile/common/textFileService.js";
import type { EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import type { IEditorPane } from "../../../workbench/browser/parts/editor/editorPane.js";
import { EditorPaneVisibility } from "../../../workbench/browser/parts/editor/editorPane.js";
import { affectsMonacoEditorFontConfiguration, readMonacoEditorFontSettings } from "../common/config/editorConfiguration.js";
import { MONACO_EDITOR_ID } from "../common/monacoEditorInput.js";
import { acquireMonacoModel, type IMonacoModelReference } from "./monacoModelService.js";

/** Browser host for the customizable Monaco editor subsystem. */
export class MonacoEditorPane extends DisposableOwner
  implements IEditorPane {
  readonly id = MONACO_EDITOR_ID;

  private _container: HTMLDivElement | undefined;
  private _editor: monaco.editor.IStandaloneCodeEditor | undefined;
  private model: monaco.editor.ITextModel | undefined;
  private modelReference: IMonacoModelReference | undefined;
  private dimension: IDimension = { width: 0, height: 0 };
  private readonly configurationService: IConfigurationService | undefined;

  constructor(private readonly textFiles: ITextFileService, configurationService?: IConfigurationService) {
    super();
    if (!textFiles || typeof textFiles.resolve !== "function") {
      throw new TypeError("Monaco editor pane requires a text file service");
    }
    this.configurationService = configurationService;
    if (this.configurationService) {
      this.own(this.configurationService.onDidChangeConfiguration(
        (event) => {
          if (affectsMonacoEditorFontConfiguration(event)) {
            this.applyFontConfiguration();
          }
        },
      ));
    }
  }

  create(parent: HTMLElement): void {
    if (this._container) {
      throw new ReferenceError("MonacoEditorPane has already been created");
    }
    const container = parent.ownerDocument.createElement("div");
    container.className = "zeta-monaco-editor-pane";
    parent.append(container);
    this._container = container;
    this._editor = monaco.editor.create(container, {
      automaticLayout: false,
      ...this.fontOptions(),
      minimap: { enabled: true },
      model: null,
      scrollBeyondLastLine: false,
    });
    this.defer(() => {
      this.clearModel();
      this._editor?.dispose();
      this._editor = undefined;
      container.remove();
      this._container = undefined;
    });
  }

  async setInput(
    input: EditorInput,
    signal: AbortSignal,
  ): Promise<void> {
    const editor = this.editor;
    throwIfAborted(signal);
    const modelReference = await acquireMonacoModel(input, this.textFiles, signal);
    const model = modelReference.model;
    if (signal.aborted) {
      modelReference.dispose();
      throw abortError();
    }
    this.clearModel();
    this.model = model;
    this.modelReference = modelReference;
    editor.setModel(model);
    editor.updateOptions({
      ariaLabel: input.label ?? resourceLabel(input),
    });
    editor.layout(this.dimension);
  }

  clearInput(): void {
    this.clearModel();
  }

  layout(dimension: IDimension): void {
    this.dimension = {
      width: Math.max(0, dimension.width),
      height: Math.max(0, dimension.height),
    };
    this._editor?.layout(this.dimension);
  }

  setVisible(visibility: EditorPaneVisibility): void {
    if (!this._container) return;
    this._container.hidden = visibility === EditorPaneVisibility.Hidden;
    if (visibility === EditorPaneVisibility.Visible) {
      this._editor?.layout(this.dimension);
    }
  }

  focus(): void {
    this.editor.focus();
  }

  getValue(): string {
    return this.model?.getValue() ?? "";
  }

  setValue(value: string): void {
    const model = this.model;
    if (!model) {
      throw new ReferenceError("MonacoEditorPane has no active input");
    }
    model.setValue(value);
  }

  private clearModel(): void {
    this._editor?.setModel(null);
    this.modelReference?.dispose();
    this.modelReference = undefined;
    this.model = undefined;
  }

  private applyFontConfiguration(): void {
    this._editor?.updateOptions(this.fontOptions());
  }

  private fontOptions(): monaco.editor.IEditorOptions {
    const settings = readMonacoEditorFontSettings(
      this.configurationService,
    );
    return {
      fontFamily: settings.fontFamily,
      fontWeight: settings.fontWeight,
      fontSize: settings.fontSize,
      fontLigatures: settings.fontLigatures,
      fontVariations: settings.fontVariations,
      lineHeight: settings.lineHeight,
      letterSpacing: settings.letterSpacing,
    };
  }

  private get editor(): monaco.editor.IStandaloneCodeEditor {
    assertDefined(this._editor, new ReferenceError("MonacoEditorPane has not been created"));
    return this._editor;
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
