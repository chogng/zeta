import "./alphaChatInputEditor.css";
import { addDisposableListener, stopEvent } from "../../../../../base/browser/dom.js";
import { Emitter, type Event } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { EditorLineWrapping } from "../../../../../editor/browser/view/visualLineProjection.js";
import { CodeEditorWidget } from "../../../../../editor/browser/widget/codeEditor/codeEditorWidget.js";
import { EditorSelectionController } from "../../../../../editor/common/cursor/editorSelectionController.js";
import { LanguageCompletionService } from "../../../../../editor/common/languages/completion/languageCompletionService.js";
import { LanguageCompletionProviderRegistry } from "../../../../../editor/common/languages/completion/languageCompletionProviders.js";
import { LanguageCompletionSessionController } from "../../../../../editor/contrib/suggest/common/suggestModel.js";
import { TextSelection, TextSelectionSet } from "../../../../../editor/common/core/selection.js";
import { TextPosition, TextRange } from "../../../../../editor/common/core/text.js";
import { TextModel } from "../../../../../editor/common/model/textModel.js";
import { type ChatInputEditorOptions, type IChatInputEditor } from "./chatInputEditor.js";
import { CHAT_INPUT_LANGUAGE_ID, createAlphaChatCommandCompletionProvider } from "./alphaChatCommandCompletion.js";

const CHAT_INPUT_LINE_HEIGHT = 20;
const CHAT_INPUT_EDITOR_PADDING = Object.freeze({ top: 0, right: 0, bottom: 0, left: 0 });
const CHAT_INPUT_MIN_HEIGHT = 106;
const CHAT_INPUT_MAX_HEIGHT = 320;

/** Alpha-backed embedded editor hosted by the Chat input part. */
export class ChatInputEditor extends DisposableOwner implements IChatInputEditor {
  readonly element: HTMLDivElement;
  private readonly model = this.own(new TextModel());
  private readonly selections = this.own(new EditorSelectionController(this.model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0)))));
  private readonly editor: CodeEditorWidget;
  private readonly _onDidChange = this.own(new Emitter<string>());
  private readonly _onDidSubmit = this.own(new Emitter<void>());
  readonly onDidChange: Event<string> = this._onDidChange.event;
  readonly onDidSubmit: Event<void> = this._onDidSubmit.event;
  private height = CHAT_INPUT_MIN_HEIGHT;
  private closed = false;

  constructor(options: ChatInputEditorOptions) {
    super();
    this.element = options.container.ownerDocument.createElement("div");
    this.element.className = "zeta-alpha-chat-input-editor";
    this.element.style.height = `${this.height}px`;
    options.container.append(this.element);
    const providers = this.own(new LanguageCompletionProviderRegistry());
    this.own(providers.register(createAlphaChatCommandCompletionProvider(options.slashCommands)));
    const completions = this.own(new LanguageCompletionService(this.model, providers));
    const completionSession = this.own(new LanguageCompletionSessionController(completions.results, this.selections, { resolver: completions }));
    this.editor = this.own(new CodeEditorWidget({
      container: this.element,
      model: this.model,
      lineHeight: CHAT_INPUT_LINE_HEIGHT,
      ariaLabel: options.ariaLabel,
      placeholder: options.placeholder,
      selectionController: this.selections,
      viewport: {
        presentation: "embedded",
        focusOutlineOwner: "host",
        padding: CHAT_INPUT_EDITOR_PADDING,
        lineWrapping: EditorLineWrapping.On,
      },
      textInput: {
        completion: {
          session: completionSession,
          requests: {
            service: completions,
            languageId: CHAT_INPUT_LANGUAGE_ID,
          },
        },
      },
    }));
    const completionWidget = this.editor.textInput.completionWidget;
    if (completionWidget) this.element.append(completionWidget.element);
    this.own(this.model.onDidChange(() => {
      this.syncHeight();
      this._onDidChange.fire(this.value);
    }));
    this.own(addDisposableListener(this.editor.textInput.element, "keydown", event => {
      if (event.defaultPrevented || event.isComposing || event.key !== "Enter" || event.shiftKey) return;
      stopEvent(event);
      this._onDidSubmit.fire();
    }));
    this.defer(() => this.closed = true);
    this.defer(() => this.element.remove());
    queueMicrotask(() => {
      if (!this.closed) this.layout();
    });
  }

  get value(): string {
    return this.model.getText();
  }

  set value(value: string) {
    if (this.model.getText() === value) return;
    const range = TextRange.from(TextPosition.at(0, 0), this.model.positionAt(this.model.length));
    this.model.applyEdits([{ range, text: value }]);
    const end = this.model.positionAt(this.model.length);
    this.selections.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(end)));
  }

  focus(): void {
    this.editor.focus();
  }

  layout(): void {
    this.editor.layout({ width: Math.max(0, this.element.clientWidth), height: this.height });
  }

  private syncHeight(): void {
    const contentHeight = this.model.lineCount * CHAT_INPUT_LINE_HEIGHT + CHAT_INPUT_EDITOR_PADDING.top + CHAT_INPUT_EDITOR_PADDING.bottom;
    const height = Math.min(CHAT_INPUT_MAX_HEIGHT, Math.max(CHAT_INPUT_MIN_HEIGHT, contentHeight));
    if (height === this.height) return;
    this.height = height;
    this.element.style.height = `${height}px`;
    this.layout();
  }
}
