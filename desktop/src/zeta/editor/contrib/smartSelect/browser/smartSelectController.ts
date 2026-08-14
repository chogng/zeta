import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { isCancellationError } from "../../../../base/common/cancellation.js";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type RustSyntaxFactsService } from "../../../browser/services/rustSyntaxFactsService.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type TextSelectionSet } from "../../../common/core/selection.js";
import { type TextRange, type TextSnapshot } from "../../../common/core/text.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { expandSmartSelection } from "../common/smartSelect.js";

/** Routes the editor smart-select shortcut into the DOM-free range expansion policy. */
export class SmartSelectController extends DisposableOwner {
  private readonly history: TextSelectionSet[] = [];
  private request: AbortController | undefined;

  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly languageId: string, private readonly syntaxFacts: RustSyntaxFactsService | undefined, private readonly wordPattern: (() => RegExp | undefined) | undefined, private readonly onError: (error: unknown) => void) {
    super();
    if (viewport.textModel !== selections.textModel) throw new TypeError("Aster smart select dependencies must share a text model");
    this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event), true));
    this.own(selections.onDidChange(change => {
      if (change.reason !== "explicit" && change.reason !== "cursorOperation") this.history.length = 0;
    }));
    this.defer(() => this.request?.abort());
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.altKey || (!event.ctrlKey && !event.metaKey) || !event.shiftKey) return;
    if (event.key === "ArrowRight") {
      stopEvent(event, { immediate: true });
      const before = this.selections.selections;
      this.request?.abort();
      this.request = undefined;
      if (!this.syntaxFacts || before.selections.every(selection => selection.collapsed)) {
        this.commitExpansion(before, this.viewport.textModel.createSnapshot(), []);
        return;
      }
      const request = this.request = new AbortController();
      const snapshot = this.viewport.textModel.createSnapshot();
      void this.expand(request, before, snapshot);
    } else if (event.key === "ArrowLeft") {
      stopEvent(event, { immediate: true });
      this.request?.abort();
      this.request = undefined;
      const previous = this.history.pop();
      if (previous) this.selections.setSelections(previous);
      this.viewport.revealPosition(this.selections.selections.primary.active);
    }
  }

  private async expand(request: AbortController, before: TextSelectionSet, snapshot: TextSnapshot): Promise<void> {
    try {
      const syntaxRanges = await this.syntaxFacts?.selectionRanges(this.languageId, snapshot, before.selections.map(selection => selection.range), request.signal) ?? [];
      if (request.signal.aborted || this.request !== request) return;
      this.commitExpansion(before, snapshot, syntaxRanges);
    } catch (error) {
      if (!isCancellationError(error)) {
        this.onError(error);
        if (this.request === request) this.commitExpansion(before, snapshot, []);
      }
    } finally {
      if (this.request === request) this.request = undefined;
    }
  }

  private commitExpansion(before: TextSelectionSet, snapshot: TextSnapshot, syntaxRanges: readonly TextRange[]): void {
    if (this.viewport.textModel.version !== snapshot.version || !this.selections.selections.equals(before)) return;
    this.history.push(before);
    this.selections.setSelections(before.map(selection => expandSmartSelection(this.viewport.textModel, selection, this.wordPattern?.(), syntaxRanges)));
    this.viewport.revealPosition(this.selections.selections.primary.active);
  }
}

registerEditorContribution({
  id: "editor.contrib.smartSelect",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new SmartSelectController(context.textInput.element, context.viewport, context.selections, context.languageId, context.getOptionalCapability(TextEditorCapability.rustSyntaxFacts), () => context.configurations.getLanguageConfiguration(context.languageId).wordPattern, context.onLanguageError));
  },
});
