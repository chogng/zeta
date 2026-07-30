import "./media/monacoChatInputEditor.css";
import "./monacoEnvironment.js";
import * as monaco from "monaco-editor";
import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { ChatInputEditorOptions, IChatInputEditor } from "../../../workbench/contrib/chat/browser/input/chatInputEditor.js";

const CHAT_INPUT_MIN_HEIGHT = 62;
const CHAT_INPUT_MAX_HEIGHT = 250;

/** Monaco-backed plaintext editor embedded inside the Chat composer. */
export class MonacoChatInputEditor extends DisposableOwner implements IChatInputEditor {
  readonly element: HTMLDivElement;
  readonly #editor: monaco.editor.IStandaloneCodeEditor;
  readonly #model: monaco.editor.ITextModel;
  readonly #onDidChange = this.own(new Emitter<string>());
  readonly #onDidSubmit = this.own(new Emitter<void>());
  readonly onDidChange: Event<string> = this.#onDidChange.event;
  readonly onDidSubmit: Event<void> = this.#onDidSubmit.event;
  #fontFamily = "sans-serif";
  #height = CHAT_INPUT_MIN_HEIGHT;
  #disposed = false;

  constructor(options: ChatInputEditorOptions) {
    super();
    this.element = options.container.ownerDocument.createElement("div");
    this.element.className = "zeta-monaco-chat-input-editor";
    this.element.style.height = `${this.#height}px`;
    options.container.append(this.element);
    this.#model = monaco.editor.createModel("", "plaintext");
    this.#editor = monaco.editor.create(this.element, {
      ariaLabel: options.ariaLabel,
      automaticLayout: true,
      bracketPairColorization: { enabled: false },
      contextmenu: true,
      folding: false,
      fontFamily: this.#fontFamily,
      fontSize: 13,
      glyphMargin: false,
      guides: { indentation: false },
      hideCursorInOverviewRuler: true,
      lineDecorationsWidth: 0,
      lineHeight: 20,
      lineNumbers: "off",
      lineNumbersMinChars: 0,
      minimap: { enabled: false },
      model: this.#model,
      overviewRulerLanes: 0,
      padding: { top: 8, bottom: 8 },
      placeholder: options.placeholder,
      quickSuggestions: false,
      renderLineHighlight: "none",
      scrollBeyondLastLine: false,
      scrollbar: {
        horizontal: "hidden",
        useShadows: false,
        vertical: "auto",
        verticalScrollbarSize: 6,
      },
      stickyScroll: { enabled: false },
      wordWrap: "on",
      wrappingStrategy: "advanced",
    });
    const contentListener = this.#editor.onDidChangeModelContent(() => {
      this.#syncHeight();
      this.#onDidChange.fire(this.value);
    });
    const sizeListener = this.#editor.onDidContentSizeChange((event) => {
      if (event.contentHeightChanged) this.#syncHeight(event.contentHeight);
    });
    const keyListener = this.#editor.onKeyDown((event) => {
      if (event.keyCode !== monaco.KeyCode.Enter || event.browserEvent.isComposing) return;
      event.preventDefault();
      event.stopPropagation();
      if (event.shiftKey) {
        this.#insertLineBreak();
        return;
      }
      this.#onDidSubmit.fire();
    });
    this.defer(() => {
      this.#disposed = true;
      keyListener.dispose();
      sizeListener.dispose();
      contentListener.dispose();
      this.#editor.dispose();
      this.#model.dispose();
      this.element.remove();
    });
    queueMicrotask(() => {
      if (!this.#disposed) this.layout();
    });
  }

  get value(): string {
    return this.#model.getValue();
  }

  set value(value: string) {
    if (this.#model.getValue() === value) return;
    this.#model.setValue(value);
  }

  focus(): void {
    this.#editor.focus();
  }

  layout(): void {
    if (this.#disposed) return;
    const fontFamily = this.element.ownerDocument.defaultView?.getComputedStyle(this.element).fontFamily;
    if (fontFamily && fontFamily !== this.#fontFamily) {
      this.#fontFamily = fontFamily;
      this.#editor.updateOptions({ fontFamily });
    }
    this.#editor.layout();
    this.#syncHeight();
  }

  #syncHeight(contentHeight = this.#editor.getContentHeight()): void {
    const height = Math.min(CHAT_INPUT_MAX_HEIGHT, Math.max(CHAT_INPUT_MIN_HEIGHT, contentHeight));
    if (height === this.#height) return;
    this.#height = height;
    this.element.style.height = `${height}px`;
    this.#editor.layout({
      width: Math.max(0, this.element.clientWidth),
      height,
    });
  }

  #insertLineBreak(): void {
    const selection = this.#editor.getSelection();
    if (!selection) return;
    const cursor = new monaco.Selection(
      selection.startLineNumber + 1,
      1,
      selection.startLineNumber + 1,
      1,
    );
    this.#editor.executeEdits("chat.input.newline", [{
      range: selection,
      text: "\n",
      forceMoveMarkers: true,
    }], [cursor]);
  }
}
