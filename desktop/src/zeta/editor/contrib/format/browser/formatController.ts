import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { createFormattingCommand, type FormatService, type LanguageFormattingOptions } from "../common/formatCommands.js";

export interface FormatControllerOptions {
  readonly formattingOptions?: LanguageFormattingOptions;
  readonly onError?: (error: unknown) => void;
}

/** Routes the editor format shortcut into the Aster formatting service and command layer. */
export class FormatController extends DisposableOwner {
  private readonly options: LanguageFormattingOptions;
  private readonly onError: (error: unknown) => void;

  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly service: FormatService, private readonly languageId: string, options: FormatControllerOptions = {}) {
    super();
    if (viewport.textModel !== selections.textModel) throw new TypeError("Aster format dependencies must share one text model");
    this.options = options.formattingOptions ?? { tabSize: 4, insertSpaces: true };
    this.onError = options.onError ?? (error => console.error("Aster formatting failed", error));
    this.own(addDisposableListener(input, "keydown", event => {
      if (event.defaultPrevented || event.isComposing || event.altKey || (!event.ctrlKey && !event.metaKey) || !event.shiftKey || event.key.toLowerCase() !== "i") return;
      stopEvent(event);
      void this.formatDocument();
    }));
  }

  async formatDocument(onError = this.onError): Promise<void> {
    try {
      const edits = await this.service.provideDocumentFormattingEdits(this.languageId, this.options);
      const command = createFormattingCommand(this.viewport.textModel, this.selections.selections, edits);
      if (command) this.selections.execute(command);
    } catch (error) {
      onError(error);
    }
  }
}

registerEditorContribution({ id: "editor.contrib.format", install: context => {
  if (context.kind !== "text") return;
  const service = context.own(context.languageFeaturesService.createFormatService(context.model, context.options.input.resource));
  context.own(new FormatController(context.textInput.element, context.viewport, context.selections, service, context.languageId, {
    formattingOptions: { tabSize: context.options.indentation?.tabSize ?? 4, insertSpaces: context.options.indentation?.kind !== "tabs" },
    onError: context.onLanguageError,
  }));
} });
