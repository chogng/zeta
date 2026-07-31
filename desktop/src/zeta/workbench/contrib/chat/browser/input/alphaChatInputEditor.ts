import "./alphaChatInputEditor.css";
import { addDisposableListener, stopEvent } from "../../../../../base/browser/dom.js";
import { Emitter, type Event } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { AlphaEditorViewport } from "../../../../../editor/alpha/browser/alphaEditorViewport.js";
import { AlphaKeyboardNavigationController } from "../../../../../editor/alpha/browser/keyboardNavigationController.js";
import { AlphaPointerSelectionController } from "../../../../../editor/alpha/browser/pointerSelectionController.js";
import { AlphaTextInputController } from "../../../../../editor/alpha/browser/textInputController.js";
import { EditorSelectionController } from "../../../../../editor/alpha/common/editorSelectionController.js";
import { LanguageCompletionService } from "../../../../../editor/alpha/common/languageCompletionService.js";
import { LanguageCompletionProviderRegistry } from "../../../../../editor/alpha/common/languageCompletionProviders.js";
import { LanguageCompletionSessionController } from "../../../../../editor/alpha/common/languageCompletionSession.js";
import { TextSelection, TextSelectionSet } from "../../../../../editor/alpha/common/selection.js";
import { TextPosition, TextRange } from "../../../../../editor/alpha/common/text.js";
import { TextModel } from "../../../../../editor/alpha/common/textModel.js";
import { type ChatInputEditorOptions, type IChatInputEditor } from "./chatInputEditor.js";
import { CHAT_INPUT_LANGUAGE_ID, createAlphaChatCommandCompletionProvider } from "./alphaChatCommandCompletion.js";

const CHAT_INPUT_LINE_HEIGHT = 20;
const CHAT_INPUT_VERTICAL_PADDING = 16;
const CHAT_INPUT_MIN_HEIGHT = 62;
const CHAT_INPUT_MAX_HEIGHT = 250;

/** Alpha-backed embedded editor used by the Chat composer. */
export class AlphaChatInputEditor extends DisposableOwner implements IChatInputEditor {
  readonly element: HTMLDivElement;
  private readonly model = this.own(new TextModel());
  private readonly selections = this.own(new EditorSelectionController(this.model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0)))));
  private readonly viewport: AlphaEditorViewport;
  private readonly input: AlphaTextInputController;
  private readonly placeholder: HTMLDivElement;
  private readonly _onDidChange = this.own(new Emitter<string>());
  private readonly _onDidSubmit = this.own(new Emitter<void>());
  readonly onDidChange: Event<string> = this._onDidChange.event;
  readonly onDidSubmit: Event<void> = this._onDidSubmit.event;
  private height = CHAT_INPUT_MIN_HEIGHT;

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
    this.viewport = this.own(new AlphaEditorViewport({
      container: this.element,
      model: this.model,
      lineHeight: CHAT_INPUT_LINE_HEIGHT,
      ariaLabel: options.ariaLabel,
      selectionController: this.selections,
      presentation: "embedded",
    }));
    this.input = this.own(new AlphaTextInputController(this.viewport, this.selections, {
      ariaLabel: options.ariaLabel,
      completion: {
        session: completionSession,
        requests: {
          service: completions,
          languageId: CHAT_INPUT_LANGUAGE_ID,
        },
      },
    }));
    this.own(new AlphaKeyboardNavigationController(this.viewport, this.selections));
    this.own(new AlphaPointerSelectionController(this.viewport, this.selections));
    const completionWidget = this.input.completionWidget;
    if (completionWidget) this.element.append(completionWidget.element);
    this.placeholder = options.container.ownerDocument.createElement("div");
    this.placeholder.className = "zeta-alpha-chat-input-placeholder";
    this.placeholder.textContent = options.placeholder;
    this.placeholder.setAttribute("aria-hidden", "true");
    this.element.append(this.placeholder);
    this.own(this.model.onDidChange(() => {
      this.placeholder.hidden = this.model.getText().length > 0;
      this.syncHeight();
      this._onDidChange.fire(this.value);
    }));
    this.own(addDisposableListener(this.input.element, "keydown", event => {
      if (event.defaultPrevented || event.isComposing || event.key !== "Enter" || event.shiftKey) return;
      stopEvent(event);
      this._onDidSubmit.fire();
    }));
    this.defer(() => this.element.remove());
    queueMicrotask(() => this.layout());
  }

  get value(): string {
    return this.model.getText();
  }

  set value(value: string) {
    if (this.model.getText() === value) return;
    const range = TextRange.from(TextPosition.at(0, 0), this.model.positionAt(this.model.getText().length));
    this.model.applyEdits([{ range, text: value }]);
    const end = this.model.positionAt(value.length);
    this.selections.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(end)));
  }

  focus(): void {
    this.input.focus();
  }

  layout(): void {
    this.viewport.layout({ width: Math.max(0, this.element.clientWidth), height: this.height });
  }

  private syncHeight(): void {
    const contentHeight = this.model.lineCount * CHAT_INPUT_LINE_HEIGHT + CHAT_INPUT_VERTICAL_PADDING;
    const height = Math.min(CHAT_INPUT_MAX_HEIGHT, Math.max(CHAT_INPUT_MIN_HEIGHT, contentHeight));
    if (height === this.height) return;
    this.height = height;
    this.element.style.height = `${height}px`;
    this.layout();
  }
}
