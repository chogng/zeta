import "./media/codeAction.css";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createEditorEditCommand } from "../../../common/commands/editorCommand.js";
import { TextRange } from "../../../common/core/text.js";
import { type LanguageDiagnostic } from "../../../common/languages/languageResults.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { type CodeActionService, type LanguageCodeAction } from "../common/codeAction.js";
import { type LanguageWorkspaceEdit } from "../../../common/languages/languageWorkspaceEdit.js";

/** Owns the editor-local code-action picker and routes selected edits through cursor commands. */
export class CodeActionController extends DisposableOwner {
  private readonly element: HTMLDivElement;
  private readonly actionListeners = this.own(new ResettableDisposableGroup());
  private request: AbortController | undefined;
  private actions: readonly LanguageCodeAction[] = [];
  private actionRange: TextRange | undefined;
  private actionDiagnostics: readonly LanguageDiagnostic[] = [];

  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly service: CodeActionService, private readonly diagnostics: TextDecorationCollection<LanguageDiagnostic>, private readonly languageId: string, private readonly resource: URI, private readonly applyWorkspaceEdit: ((edit: LanguageWorkspaceEdit) => void | Promise<void>) | undefined, private readonly onError: (error: unknown) => void = error => console.error("Editor code action failed", error)) {
    super();
    if (viewport.textModel !== selections.textModel || diagnostics.textModel !== selections.textModel) throw new TypeError("Aster code action dependencies must share one text model");
    const ownerDocument = viewport.element.ownerDocument;
    this.element = ownerDocument.createElement("div");
    this.element.className = "aster-editor-code-action";
    this.element.hidden = true;
    this.element.setAttribute("role", "menu");
    viewport.element.append(this.element);
    this.defer(() => this.element.remove());
    this.own(addDisposableListener(input, "keydown", event => {
      if (event.defaultPrevented || event.isComposing || event.altKey || (!event.ctrlKey && !event.metaKey) || event.key !== ".") return;
      stopEvent(event);
      void this.open();
    }));
    this.own(addDisposableListener(this.element, "keydown", event => {
      if (event.key !== "Escape") return;
      stopEvent(event);
      this.close();
    }));
    this.own(viewport.textModel.onDidChange(() => this.close()));
  }

  private async open(): Promise<void> {
    this.cancelRequest();
    const request = this.request = new AbortController();
    try {
      const range = this.selections.selections.primary.range;
      const diagnostics = this.diagnostics.decorations.filter(decoration => decoration.range.intersectsOrTouches(range)).map(decoration => decoration.metadata);
      const actions = await this.service.provideCodeActions(this.languageId, range.empty ? TextRange.emptyAt(range.start) : range, diagnostics, undefined, request.signal);
      if (request.signal.aborted || actions.length === 0) {
        if (!request.signal.aborted) this.viewport.announceAccessibilityStatus("No code actions available.");
        return;
      }
      this.actions = actions;
      this.actionRange = range;
      this.actionDiagnostics = diagnostics;
      this.render();
    } catch (error) {
      if (!request.signal.aborted) this.onError(error);
    }
  }

  private render(): void {
    this.actionListeners.clear();
    this.element.replaceChildren(...this.actions.map((action, index) => {
      const button = this.element.ownerDocument.createElement("button");
      button.type = "button";
      button.setAttribute("role", "menuitem");
      button.textContent = action.disabledReason ? `${action.title} (${action.disabledReason})` : action.title;
      button.disabled = action.disabledReason !== undefined;
      this.actionListeners.add(addDisposableListener(button, "click", () => void this.apply(index)));
      return button;
    }));
    const coordinates = this.viewport.getPositionContentCoordinates(this.selections.selections.primary.range.start);
    this.element.style.left = `${Math.max(8, coordinates.left - this.viewport.viewportLayout.scrollPosition.left)}px`;
    this.element.style.top = `${Math.max(8, coordinates.top - this.viewport.viewportLayout.scrollPosition.top + coordinates.height + 4)}px`;
    this.element.hidden = false;
    (this.element.querySelector("button:not(:disabled)") as HTMLButtonElement | null)?.focus({ preventScroll: true });
  }

  private async apply(index: number): Promise<void> {
    const action = this.actions[index];
    const range = this.actionRange;
    if (!action || !range) return;
    try {
      const resolved = action.edit ? action : await this.service.resolveCodeAction(this.languageId, range, action, this.actionDiagnostics);
      if (!resolved.edit) {
        this.viewport.announceAccessibilityStatus(`${resolved.title} has no text edit.`);
        this.close();
        return;
      }
      if (this.applyWorkspaceEdit) {
        await this.applyWorkspaceEdit(resolved.edit);
      } else {
        const documentEdit = resolved.edit.entries.find(edit => edit.kind === "textDocument" && edit.resource.toString() === this.resource.toString());
        if (resolved.edit.entries.length !== 1 || !documentEdit || documentEdit.kind !== "textDocument") throw new Error("This editor host cannot apply a multi-resource code action");
        const command = createEditorEditCommand(this.viewport.textModel, this.selections.selections, documentEdit.edits);
        if (command) this.selections.execute(command);
      }
      this.close();
    } catch (error) {
      this.onError(error);
    }
  }

  private close(): void {
    this.cancelRequest();
    this.actions = [];
    this.actionRange = undefined;
    this.actionDiagnostics = [];
    this.element.hidden = true;
    this.actionListeners.clear();
    this.element.replaceChildren();
    this.input.focus({ preventScroll: true });
  }

  private cancelRequest(): void {
    this.request?.abort();
    this.request = undefined;
  }
}
