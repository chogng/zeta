import "./media/inlineCompletions.css";
import { addDisposableListener, stopEvent } from "../../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { createEditorEditCommand } from "../../../common/commands/editorCommand.js";
import { TextRange } from "../../../common/core/text.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type InlineCompletionsService, type LanguageInlineCompletionItem } from "../common/inlineCompletions.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Owns ghost-text projection and explicit acceptance of one inline completion. */
export class InlineCompletionsController extends DisposableOwner {
  private readonly element: HTMLSpanElement;
  private request: AbortController | undefined;
  private item: LanguageInlineCompletionItem | undefined;

  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly service: InlineCompletionsService, private readonly languageId: string, private readonly onError: (error: unknown) => void = error => console.error("Alpha inline completion failed", error)) {
    super();
    const element = this.element = viewport.element.ownerDocument.createElement("span");
    element.className = "zeta-alpha-editor-inline-completion";
    element.hidden = true;
    viewport.element.append(element);
    this.defer(() => element.remove());
    this.own(addDisposableListener(input, "keydown", event => {
      if (event.defaultPrevented || event.isComposing || !event.ctrlKey || !event.altKey || event.key !== " ") return;
      stopEvent(event);
      void this.refresh("explicit");
    }));
    this.own(addDisposableListener(input, "keydown", event => {
      if (event.defaultPrevented || event.isComposing || !this.item || event.key !== "Enter" || !event.altKey) return;
      stopEvent(event);
      this.accept();
    }));
    this.own(selections.onDidChange(() => this.clear()));
    this.own(viewport.onDidChangeLayout(() => this.render()));
    this.own(viewport.textModel.onDidChange(() => this.clear()));
  }

  private async refresh(triggerKind: "automatic" | "explicit"): Promise<void> {
    const selection = this.selections.selections.primary;
    if (!selection.range.empty) return;
    this.request?.abort();
    const request = this.request = new AbortController();
    try {
      const items = await this.service.provideInlineCompletions(this.languageId, selection.active, triggerKind, request.signal);
      if (request.signal.aborted) return;
      this.item = items[0];
      this.render();
    } catch (error) {
      if (!request.signal.aborted) this.onError(error);
    }
  }

  private render(): void {
    const item = this.item;
    if (!item) {
      this.element.hidden = true;
      return;
    }
    const selection = this.selections.selections.primary;
    const range = item.range ?? TextRange.emptyAt(selection.active);
    const coordinates = this.viewport.getPositionContentCoordinates(range.start);
    const scroll = this.viewport.viewportLayout.scrollPosition;
    this.element.textContent = item.insertText;
    this.element.style.left = `${coordinates.left - scroll.left}px`;
    this.element.style.top = `${coordinates.top - scroll.top}px`;
    this.element.hidden = false;
  }

  private accept(): void {
    const item = this.item;
    if (!item) return;
    const selection = this.selections.selections.primary;
    const edits = [...(item.additionalTextEdits ?? []), { range: item.range ?? TextRange.emptyAt(selection.active), text: item.insertText }].sort((left, right) => left.range.start.compareTo(right.range.start));
    const command = createEditorEditCommand(this.viewport.textModel, this.selections.selections, edits);
    if (command) this.selections.execute(command);
    this.clear();
  }

  private clear(): void {
    this.request?.abort();
    this.request = undefined;
    this.item = undefined;
    this.element.hidden = true;
    this.element.textContent = "";
  }
}
