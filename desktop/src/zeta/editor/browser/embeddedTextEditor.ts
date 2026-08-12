import { type IDimension } from "../../base/browser/geometry.js";
import { Emitter } from "../../base/common/event.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import { type ITextMateService } from "../../workbench/services/textMate/common/textMateService.js";
import { type IEmbeddedTextEditor, type EmbeddedTextEditorOptions, type IEmbeddedTextEditorFactory } from "../../workbench/browser/parts/editor/embeddedTextEditor.js";
import { type ILanguageFeaturesService } from "../common/services/languageService.js";
import { type TextResourceChangeEvent, type TextResourceContent, type TextResourceResolveRequest, type TextResourceSaveRequest, type ITextResourceStore } from "../common/services/textResourceStore.js";
import { BrowserTextModelService } from "./services/browserTextModelService.js";
import { EditorPane } from "./codeEditorPane.js";
import { EditorPart } from "./editorPart.js";

/** Options shared by Alpha CodeEditorWidget projections mounted in TextEditorWidget textBlock nodes. */
export interface EmbeddedTextEditorFactoryOptions {
  readonly textMateService?: ITextMateService;
  readonly languageFeaturesService?: ILanguageFeaturesService;
}

/** Creates Alpha line editors for TextEditorWidget textBlock projections. */
export class EmbeddedTextEditorFactory implements IEmbeddedTextEditorFactory {
  constructor(private readonly options: EmbeddedTextEditorFactoryOptions = {}) {}

  create(options: EmbeddedTextEditorOptions): IEmbeddedTextEditor {
    return new EmbeddedTextEditor(options, this.options);
  }
}

class EmbeddedTextEditor extends DisposableOwner implements IEmbeddedTextEditor {
  private readonly changeEmitter = this.own(new Emitter<string>());
  readonly onDidChange = this.changeEmitter.event;
  private readonly resourceStore: EmbeddedTextResourceStore;
  private readonly modelService: BrowserTextModelService;
  private readonly pane: EditorPane;
  private readonly input: EmbeddedTextEditorInput;
  private editorPart: EditorPart | undefined;
  private parent: HTMLElement | undefined;
  private pendingValue: string;
  private started = false;

  constructor(options: EmbeddedTextEditorOptions, factoryOptions: EmbeddedTextEditorFactoryOptions) {
    super();
    this.pendingValue = options.initialText;
    this.resourceStore = this.own(new EmbeddedTextResourceStore(options.resource, options.initialText));
    this.modelService = this.own(new BrowserTextModelService(this.resourceStore));
    this.input = {
      resource: options.resource,
      label: options.label,
      languageId: options.languageId ?? "plaintext",
      readOnly: options.readOnly,
      initialText: options.initialText,
    };
    this.pane = this.own(new EditorPane(this.resourceStore, {
      modelService: this.modelService,
      textMateService: factoryOptions.textMateService,
      languageFeaturesService: factoryOptions.languageFeaturesService,
      createPart: partOptions => {
        const editorPart = new EditorPart({ ...partOptions, presentation: "embedded" });
        this.editorPart = editorPart;
        this.own(editorPart.onDidChange(() => {
          this.pendingValue = editorPart.getValue();
          this.changeEmitter.fire(this.pendingValue);
        }));
        return editorPart;
      },
    }));
  }

  create(parent: HTMLElement): void {
    if (this.started) throw new ReferenceError("Alpha embedded editor has already been created");
    this.started = true;
    this.parent = parent;
    this.pane.create(parent);
    const controller = new AbortController();
    this.defer(() => controller.abort());
    void this.pane.setInput(this.input, controller.signal).catch(error => {
      if (!controller.signal.aborted) console.error("Alpha embedded editor failed to initialize", error);
    });
  }

  setValue(value: string): void {
    this.pendingValue = value;
    this.editorPart?.setValue(value);
  }

  getValue(): string {
    return this.editorPart?.getValue() ?? this.pendingValue;
  }

  layout(dimension: IDimension): void {
    this.pane.layout(dimension);
  }

  focus(): void {
    this.pane.focus();
  }

  override dispose(): void {
    this.parent = undefined;
    super.dispose();
  }
}

class EmbeddedTextResourceStore extends DisposableOwner implements ITextResourceStore {
  private readonly changeEmitter = this.own(new Emitter<TextResourceChangeEvent>());
  readonly onDidChange = this.changeEmitter.event;
  private text: string;
  private resolved = false;

  constructor(private readonly resource: EmbeddedTextEditorInput["resource"], initialText: string) {
    super();
    this.text = initialText;
  }

  async resolve(request: TextResourceResolveRequest, _signal: AbortSignal): Promise<TextResourceContent> {
    if (request.bootstrapText !== undefined && !this.resolved) this.text = request.bootstrapText;
    this.resolved = true;
    return Object.freeze({ resource: request.resource, text: this.text, revision: undefined });
  }

  async save(request: TextResourceSaveRequest, _signal: AbortSignal): Promise<{ readonly revision: string | undefined }> {
    this.text = request.text;
    this.changeEmitter.fire({ resources: [this.resource] });
    return Object.freeze({ revision: undefined });
  }
}

interface EmbeddedTextEditorInput {
  readonly resource: EmbeddedTextEditorOptions["resource"];
  readonly label: string;
  readonly languageId: string;
  readonly readOnly?: boolean;
  readonly initialText: string;
}
